use super::super::super::super::arguments::Mutation;
use super::*;

impl Snapshots {
    pub(super) fn call(
        &mut self,
        before: Value,
        call: &Mutation,
        definitions: &definitions::Definitions,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Value> {
        if call.roots.len() > crate::check::dependencies::references::MAX_ROOTS || !flow.spend(5) {
            return Err(budget(span));
        }
        let Value::Known(id) = before else {
            return Ok(Value::Missing);
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        if state.owner() != call.owner {
            return Err(invalid(span));
        }
        if state.reach == FALSE {
            return Ok(before);
        }
        let input = self.rows.get(&call.input).ok_or_else(|| invalid(span))?;
        if input.owner != call.owner || input.after != before {
            return Err(invalid(span));
        }
        if call.unknown {
            return Ok(Value::Missing);
        }
        if call.roots.is_empty() {
            return Ok(before);
        }
        let mut after = state.snapshot(flow, span, self.room())?;
        let mut prior = None;
        for &local in &call.roots {
            if !flow.spend(state.cells.depth() + 5) {
                return Err(budget(span));
            }
            if prior.is_some_and(|prior| prior >= local) {
                return Err(invalid(span));
            }
            prior = Some(local);
            let Some(location) = definitions.declarations.get(&local) else {
                return Ok(Value::Missing);
            };
            if location.version.owner != call.owner || location.version.local != local {
                return Err(invalid(span));
            }
            let Port::Operation(point) = location.port else {
                return Ok(Value::Missing);
            };
            let Some(storage) = definitions.storage.get(&point) else {
                return Ok(Value::Missing);
            };
            if storage.version != location.version {
                return Err(invalid(span));
            }
            let Some(old) = state.get(local) else {
                return Ok(Value::Missing);
            };
            old.validate(call.owner, local, flow, span)?;
            if let Data::Initialized {
                domain: Some(domain),
                ..
            } = &old.data
                && !domain.fits(&storage.declared, flow, span)?
            {
                return Err(invalid(span));
            }
            if storage.declared.weight() > self.room() || !flow.spend(storage.declared.weight() + 1)
            {
                return Err(budget(span));
            }
            let cell = Cell {
                data: if matches!(old.data, Data::Initialized { .. }) {
                    Data::Initialized {
                        version: None,
                        domain: Some(storage.declared.clone()),
                        origin: None,
                    }
                } else {
                    old.data.clone()
                },
                control: old.control || call.control,
            };
            after.put(local, &cell, flow, span, self.room())?;
        }
        self.keep(after, span)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
