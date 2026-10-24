use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct Slot {
    pub(in super::super) owner: usize,
    pub(in super::super) declared: domain::Domain,
    pub(in super::super) control: Option<bool>,
}

impl Checker {
    pub(in super::super) fn scalar_slot_observations<'a, 'd>(
        &mut self,
        reports: &'a Reports,
        graph: &graph::Graph<'_>,
        mut observations: Observations<'a, 'd>,
        span: Span,
        parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        if observations.scopes.len() > MAX_EDGES || !self.flow.spend(observations.scopes.len() + 1)
        {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        for (&block, scope) in &observations.scopes {
            if !self.flow.spend(
                self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.blocks.len().checked_ilog2().unwrap_or(0) as usize
                    + 6,
            ) {
                return Err(budget(span));
            }
            let body = self.bodies.get(&block).ok_or_else(|| invalid(span))?;
            let (owner, observed) = reports.blocks.get(&block).ok_or_else(|| invalid(span))?;
            if scope.owner != *owner
                || scope.owner != body.owner
                || scope.parent != body.parent
                || scope.parent != observed.parent
                || Some(scope.scalar) != body.scalar
                || observed.scalar != body.scalar
                || scope.scalar.shape() != Some(body.completion.result)
                || observed.completion != body.completion
                || scope.normal != observed.normal
                || scope.result != observed.result
            {
                return Err(invalid(span));
            }
            self.validate_result_layout(block, span)?;
            let port = Port::BlockEntry(block);
            let Some(node) = graph.nodes.get(&port) else {
                continue;
            };
            if node.site != self.scalar_port_site(port, scope.owner, span)? {
                return Err(invalid(span));
            }
            let declared = scope.declared(&mut self.flow, span)?;
            parts = parts
                .checked_sub(1 + declared.weight())
                .ok_or_else(|| budget(span))?;
            if observations
                .slots
                .insert(
                    block,
                    Slot {
                        owner: scope.owner,
                        declared,
                        control: scope.control,
                    },
                )
                .is_some()
            {
                return Err(invalid(span));
            }
        }
        Ok((observations, parts))
    }
}

#[cfg(test)]
mod tests;
