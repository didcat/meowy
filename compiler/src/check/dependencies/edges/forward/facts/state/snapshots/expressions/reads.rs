use super::*;
use versions::Definition;

impl Snapshots {
    pub(super) fn read(
        &self,
        state: &State,
        local: hir::LocalId,
        control: bool,
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<Scalar>> {
        if !flow.spend(state.cells.len().checked_ilog2().unwrap_or(0) as usize + 3) {
            return Err(budget(span));
        }
        let declared = observations
            .declared
            .get(&local)
            .ok_or_else(|| invalid(span))?;
        let Some(cell) = state.get(local) else {
            return Ok(None);
        };
        cell.validate(state.owner(), local, flow, span)?;
        let Data::Initialized {
            version,
            domain,
            origin,
        } = &cell.data
        else {
            return Ok(None);
        };
        if let Some(version) = version {
            let (map, key) = match version.definition {
                Definition::Write(point) => (&observations.definitions.writes, point),
                _ => (&observations.definitions.declarations, local),
            };
            if !flow.spend(map.len().checked_ilog2().unwrap_or(0) as usize + 3) {
                return Err(budget(span));
            }
            let location = map.get(&key).ok_or_else(|| invalid(span))?;
            let row = self.rows.get(&location.port).ok_or_else(|| invalid(span))?;
            if location.version != *version || row.owner != state.owner() {
                return Err(invalid(span));
            }
            let Value::Known(id) = row.after else {
                return Err(invalid(span));
            };
            let defined = self.states.get(id).ok_or_else(|| invalid(span))?;
            if !flow.spend(defined.cells.len().checked_ilog2().unwrap_or(0) as usize + 1) {
                return Err(budget(span));
            }
            if !matches!(defined.get(local).map(|cell| &cell.data),
                Some(Data::Initialized { version: Some(found), .. }) if found == version)
            {
                return Err(invalid(span));
            }
        }
        let Some(domain) = domain else {
            return Ok(None);
        };
        if domain.join(declared, flow, span)? != *declared {
            return Err(invalid(span));
        }
        let weight =
            1 + domain.weight() + usize::from(version.is_some()) + usize::from(origin.is_some());
        if weight > self.parts || !flow.spend(weight) {
            return Err(budget(span));
        }
        Ok(Some(Scalar {
            domain: domain.clone(),
            version: *version,
            origin: *origin,
            control: control || cell.control,
        }))
    }
}

#[cfg(test)]
mod tests;
