use super::*;

impl Snapshots {
    pub(super) fn block_expression(
        &mut self,
        point: PointId,
        forward: &observations::Forward,
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        let value = self.block_value(point, forward, observations, flow, span)?;
        self.record_expression(
            point,
            Fact {
                owner: forward.owner,
                value,
            },
            flow,
            span,
        )
    }

    pub(self) fn block_value(
        &self,
        point: PointId,
        forward: &observations::Forward,
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<Scalar>> {
        if !flow.spend(10) {
            return Err(budget(span));
        }
        let Port::BlockResult(block) = forward.input else {
            return Err(invalid(span));
        };
        let row = self
            .rows
            .get(&Port::Normal(point))
            .ok_or_else(|| invalid(span))?;
        if row.owner != forward.owner || row.before != row.after || forward.control {
            return Err(invalid(span));
        }
        let Some(scope) = observations.scopes.get(&block) else {
            return Ok(None);
        };
        if scope.owner != forward.owner || scope.parent != Some(point) {
            return Err(invalid(span));
        }
        let normal = self.rows.get(&Port::BlockNormal(block));
        let result = self.rows.get(&forward.input);
        if scope.normal != normal.is_some()
            || scope.result != result.is_some()
            || normal.is_some_and(|normal| normal.owner != row.owner)
            || result.is_some_and(|result| {
                result.owner != row.owner
                    || result.before != normal.map_or(Value::Missing, |normal| normal.after)
                    || result.before != result.after
                    || result.control != normal.and_then(|normal| normal.control)
            })
            || row.before != result.map_or(Value::Missing, |result| result.after)
            || row.control != result.and_then(|result| result.control)
        {
            return Err(invalid(span));
        }
        let Some(slot) = observations.slots.get(&block) else {
            return Ok(None);
        };
        if slot.owner != row.owner
            || slot.control != scope.control
            || Domain::scalar(scope.kind, flow, span)?.as_ref() != Some(&slot.declared)
        {
            return Err(invalid(span));
        }
        let Value::Known(id) = row.after else {
            return Ok(None);
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        if state.owner() != row.owner {
            return Err(invalid(span));
        }
        if state.reach == FALSE {
            return Ok(Some(Scalar {
                domain: Domain::BOTTOM,
                local: None,
                version: None,
                origin: None,
                control: row.control.unwrap_or(true),
            }));
        }
        let (Some(control), Some(entry)) = (row.control, scope.control) else {
            return Ok(None);
        };
        if !flow.spend(state.slots.depth() + 1) {
            return Err(budget(span));
        }
        let Some(cell) = state.slot(block) else {
            return Ok(None);
        };
        cell.validate(row.owner, block, flow, span)?;
        let (domain, origin) = match &cell.data {
            Data::Uninitialized if slot.declared == Domain::NULL => {
                (&Domain::NULL, Some(Origin::Expression(row.owner, point)))
            }
            Data::Conditional { domain, .. } if slot.declared == Domain::NULL => (domain, None),
            Data::Initialized {
                version: Some(_), ..
            } => return Err(invalid(span)),
            Data::Initialized {
                domain: Some(domain),
                origin,
                ..
            } => (domain, *origin),
            _ => return Ok(None),
        };
        if !domain.fits(&slot.declared, flow, span)? {
            return Ok(None);
        }
        let weight = 1 + domain.weight() + usize::from(origin.is_some());
        if weight > self.parts || !flow.spend(weight) {
            return Err(budget(span));
        }
        Ok(Some(Scalar {
            domain: domain.clone(),
            local: None,
            version: None,
            origin,
            control: control || entry || cell.control,
        }))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod nulls;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod conditional;
