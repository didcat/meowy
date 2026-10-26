use super::*;

impl Snapshots {
    pub(in super::super) fn primary_expression(
        &mut self,
        port: Port,
        primary: &observations::Primary,
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        let value = self.primary_value(port, primary, observations, flow, span)?;
        let fact = Fact {
            owner: primary.owner,
            value,
        };
        let weight = fact.weight();
        if self.projections.len() >= MAX_EDGES || !flow.spend(weight + 2) {
            return Err(budget(span));
        }
        if self.projections.contains_key(&port) {
            return Err(invalid(span));
        }
        let parts = self.parts.checked_sub(weight).ok_or_else(|| budget(span))?;
        self.projections.insert(port, fact);
        self.parts = parts;
        Ok(())
    }

    pub(self) fn primary_value(
        &self,
        port: Port,
        primary: &observations::Primary,
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<Scalar>> {
        if !flow.spend(8) {
            return Err(budget(span));
        }
        if !matches!(port, Port::Projection { step: 0 | 1, .. }) {
            return Err(invalid(span));
        }
        let row = self.rows.get(&port).ok_or_else(|| invalid(span))?;
        if row.owner != primary.owner || row.before != row.after {
            return Err(invalid(span));
        }
        let Some(scope) = observations.scopes.get(&primary.block) else {
            return Ok(None);
        };
        if scope.owner != primary.owner {
            return Err(invalid(span));
        }
        let parent = scope.parent.ok_or_else(|| invalid(span))?;
        let source = self.rows.get(&Port::Normal(primary.input));
        let result = self.rows.get(&Port::Normal(parent));
        for source in [source, result].into_iter().flatten() {
            if source.owner != row.owner
                || source.before != source.after
                || source.after != row.before
                || source.control != row.control
            {
                return Err(invalid(span));
            }
        }
        if source.is_none() || result.is_none() {
            return Ok(None);
        }
        let forward = observations::Forward {
            owner: primary.owner,
            input: Port::BlockResult(primary.block),
            control: false,
        };
        self.block_value(parent, &forward, observations, flow, span)
            .map(|value| {
                value.map(|mut value| {
                    value.control |= primary.control;
                    value
                })
            })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod limits;
