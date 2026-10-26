use super::*;

impl Snapshots {
    pub(super) fn record_expression_value(
        &mut self,
        point: PointId,
        block: hir::BlockId,
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        if !flow.spend(4) {
            return Err(budget(span));
        }
        let scope = observations
            .scopes
            .get(&block)
            .ok_or_else(|| invalid(span))?;
        let port = Port::Normal(point);
        if observations.sources.contains_key(&port)
            || observations.forwarded.contains_key(&port)
            || observations.logical.contains_key(&port)
        {
            return Err(invalid(span));
        }
        let forward = observations::Forward {
            owner: scope.owner,
            input: Port::BlockResult(block),
            control: false,
        };
        let value = self.block_value(point, &forward, observations, flow, span)?;
        self.record_primary(
            point,
            block,
            Fact {
                owner: scope.owner,
                value,
            },
            flow,
            span,
        )
    }

    pub(super) fn record_primary(
        &mut self,
        point: PointId,
        block: hir::BlockId,
        fact: Fact,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        let weight = 1 + fact.weight();
        if self.records.len() >= MAX_EDGES || !flow.spend(weight + 2) {
            return Err(budget(span));
        }
        if self.records.contains_key(&point) || self.expressions.contains_key(&point) {
            return Err(invalid(span));
        }
        let parts = self.parts.checked_sub(weight).ok_or_else(|| budget(span))?;
        self.records.insert(point, (block, fact));
        self.parts = parts;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
