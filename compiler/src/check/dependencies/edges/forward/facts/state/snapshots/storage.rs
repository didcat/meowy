use super::*;
use versions::Definition;

impl Snapshots {
    pub(super) fn storage(
        &mut self,
        point: PointId,
        before: Value,
        definitions: &definitions::Definitions,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<Value>> {
        if !flow.spend(2) {
            return Err(budget(span));
        }
        let record = if definitions.records.is_empty() {
            None
        } else {
            if !flow.spend(1) {
                return Err(budget(span));
            }
            definitions.records.get(&point)
        };
        if record.is_some() && definitions.storage.contains_key(&point) {
            return Err(invalid(span));
        }
        let Some(storage) = definitions
            .storage
            .get(&point)
            .or_else(|| record.map(|record| &record.storage))
        else {
            return Ok(None);
        };
        let (site, map, key, binding) = match storage.version.definition {
            Definition::Bind(site) => {
                (site, &definitions.declarations, storage.version.local, true)
            }
            Definition::Write(site) => (site, &definitions.writes, site, false),
            Definition::Parameter(_) => return Ok(None),
        };
        if !flow.spend(3) {
            return Err(budget(span));
        }
        let location = map.get(&key).ok_or_else(|| invalid(span))?;
        if site != point
            || location.port != Port::Operation(point)
            || location.version != storage.version
        {
            return Err(invalid(span));
        }
        let Value::Known(id) = before else {
            return Ok(Some(Value::Missing));
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        if state.owner() != storage.version.owner {
            return Err(invalid(span));
        }
        if state.reach == FALSE {
            return Ok(Some(before));
        }
        if !flow.spend(state.cells.depth() + 3) {
            return Err(budget(span));
        }
        let Some(old) = state.get(storage.version.local) else {
            return Ok(Some(Value::Missing));
        };
        if binding && old.data != Data::Uninitialized {
            return Ok(Some(Value::Missing));
        }
        let Some(source) = self.rows.get(&Port::Normal(storage.input)) else {
            return Ok(Some(Value::Missing));
        };
        if source.owner != storage.version.owner || source.after != before {
            return Err(invalid(span));
        }
        let fact = if let Some(record) = record {
            if !binding {
                return Err(invalid(span));
            }
            let Some((block, fact)) = self.records.get(&storage.input) else {
                return Ok(Some(Value::Missing));
            };
            if *block != record.block || self.expressions.contains_key(&storage.input) {
                return Err(invalid(span));
            }
            Some(fact)
        } else {
            self.expressions.get(&storage.input)
        };
        if fact.is_some_and(|fact| fact.owner != storage.version.owner) {
            return Err(invalid(span));
        }
        let value = fact.and_then(|fact| fact.value.as_ref());
        let domain = value
            .map(|value| &value.domain)
            .or_else(|| record.is_none().then_some(&storage.declared));
        if let Some(domain) = domain
            && !domain.fits(&storage.declared, flow, span)?
        {
            return Err(invalid(span));
        }
        let weight = domain.map_or(0, Domain::weight);
        if weight > self.room() || !flow.spend(weight + 2) {
            return Err(budget(span));
        }
        let origin = match (domain, value.and_then(|value| value.origin)) {
            (None, _) => None,
            (_, Some(origin)) => Some(origin),
            (_, None) => Some(Origin::of_version(
                value
                    .and_then(|value| value.version)
                    .unwrap_or(storage.version),
                span,
            )?),
        };
        let cell = Cell {
            data: Data::Initialized {
                version: Some(storage.version),
                domain: domain.cloned(),
                origin,
            },
            control: old.control || storage.control || value.is_none_or(|value| value.control),
        };
        let mut after = state.snapshot(flow, span, self.room())?;
        after.put(storage.version.local, &cell, flow, span, self.room())?;
        self.keep(after, span).map(Some)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod writes;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod records;

#[cfg(test)]
mod record_limits;
