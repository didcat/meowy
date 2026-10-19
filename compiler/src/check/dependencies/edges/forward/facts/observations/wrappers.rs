use super::*;
use crate::check::dependencies::grouped::MAX_GROUPS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) struct Forward {
    pub(in super::super) owner: usize,
    pub(in super::super) input: PointId,
}

impl Checker {
    pub(super) fn scalar_wrapper_observations<'a, 'd>(
        &mut self,
        reports: &Reports,
        graph: &graph::Graph<'_>,
        mut observations: Observations<'a, 'd>,
        span: Span,
        parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        let len = self.group_inputs.len();
        if len > MAX_GROUPS || !self.flow.spend(len + 1) {
            return Err(budget(span));
        }
        let mut parts = parts
            .min(MAX_EDGES)
            .checked_sub(len)
            .ok_or_else(|| budget(span))?;
        let groups = self
            .group_inputs
            .iter()
            .map(|(&point, &group)| (point, group))
            .collect::<Vec<_>>();
        for (point, group) in groups {
            if !self
                .flow
                .spend(reports.effects.len().checked_ilog2().unwrap_or(0) as usize + 3)
            {
                return Err(budget(span));
            }
            let port = Port::Normal(point);
            if graph.nodes.contains_key(&Port::Operation(point)) {
                return Err(invalid(span));
            }
            if !graph.nodes.contains_key(&port) {
                continue;
            }
            if reports.effects.contains_key(&point) {
                return Err(invalid(span));
            }
            let input = self.qualified_group_input(point, group.owner, group, span)?;
            self.insert_scalar_forward(
                graph,
                &mut observations,
                (Family::Region, port),
                Forward {
                    owner: group.owner,
                    input,
                },
                span,
                &mut parts,
            )?;
        }
        Ok((observations, parts))
    }

    pub(self) fn insert_scalar_forward(
        &mut self,
        graph: &graph::Graph<'_>,
        observations: &mut Observations<'_, '_>,
        stage: (Family, Port),
        forward: Forward,
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        if !self.flow.spend(7) {
            return Err(budget(span));
        }
        let from = Port::Normal(forward.input);
        let edge = (stage.0, Edge::new(from, stage.1, Route::Next));
        let node = graph.nodes.get(&stage.1).ok_or_else(|| invalid(span))?;
        let [input] = node.incoming.as_slice() else {
            return Err(invalid(span));
        };
        let source = graph
            .nodes
            .get(&from)
            .map(|node| node.site)
            .or_else(|| graph.external.get(&from).copied());
        if node.site != self.scalar_port_site(stage.1, forward.owner, span)?
            || input.source != self.scalar_port_site(from, forward.owner, span)?
            || input.source.block != node.site.block
            || source != Some(input.source)
            || input.reached != graph.nodes.contains_key(&from)
            || graph.edges.get(input.edge) != Some(&edge)
        {
            return Err(invalid(span));
        }
        *parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
        if observations.forwarded.insert(stage.1, forward).is_some() {
            return Err(invalid(span));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
