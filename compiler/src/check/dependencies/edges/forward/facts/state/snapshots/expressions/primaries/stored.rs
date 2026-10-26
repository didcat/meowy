use super::*;

impl Snapshots {
    pub(super) fn stored_primary_value(
        &self,
        port: Port,
        primary: &observations::Primary,
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<Scalar>> {
        if !flow.spend(6) {
            return Err(budget(span));
        }
        let row = self.rows.get(&port).ok_or_else(|| invalid(span))?;
        let source = self.rows.get(&Port::Normal(primary.input));
        let record = self.records.get(&primary.input);
        if source.is_some_and(|source| {
            source.owner != primary.owner
                || source.before != source.after
                || source.after != row.before
                || source.control != row.control
        }) || record.is_some_and(|(block, fact)| {
            *block != primary.block || fact.owner != primary.owner || source.is_none()
        }) {
            return Err(invalid(span));
        }
        let Some(scope) = observations.scopes.get(&primary.block) else {
            return Ok(None);
        };
        if scope.owner != primary.owner {
            return Err(invalid(span));
        }
        let Value::Known(id) = row.after else {
            return Ok(None);
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        if state.owner() != primary.owner {
            return Err(invalid(span));
        }
        if state.reach == FALSE {
            return Ok(Some(Scalar {
                domain: Domain::BOTTOM,
                local: None,
                version: None,
                origin: None,
                control: row.control.unwrap_or(true) || primary.control,
            }));
        }
        let (Some(history), Some((_, fact))) = (row.control, record) else {
            return Ok(None);
        };
        let Some(value) = &fact.value else {
            return Ok(None);
        };
        let declared = scope.declared(flow, span)?;
        if !value.domain.fits(&declared, flow, span)? {
            return Ok(None);
        }
        let weight = 1 + value.domain.weight() + usize::from(value.origin.is_some());
        if weight > self.parts || !flow.spend(weight) {
            return Err(budget(span));
        }
        Ok(Some(Scalar {
            domain: value.domain.clone(),
            local: None,
            version: None,
            origin: value.origin,
            control: value.control || history || primary.control,
        }))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
