use super::super::super::arguments::Mutations;
use super::*;

impl Checker {
    pub(in super::super) fn scalar_call_observations<'a, 'd>(
        &mut self,
        reports: &Reports,
        graph: &graph::Graph<'_>,
        calls: &'d Mutations,
        mut observations: Observations<'a, 'd>,
        span: Span,
        parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        if calls.len() > MAX_EDGES || !self.flow.spend(calls.len() + 1) {
            return Err(budget(span));
        }
        let parts = parts
            .min(MAX_EDGES)
            .checked_sub(calls.len())
            .ok_or_else(|| budget(span))?;
        for (&point, call) in calls {
            if !self
                .flow
                .spend(reports.effects.len().checked_ilog2().unwrap_or(0) as usize + 9)
            {
                return Err(budget(span));
            }
            let (owner, effect) = reports.effects.get(&point).ok_or_else(|| invalid(span))?;
            self.validate_call_report(reports, point, *owner, effect, span)?;
            if !matches!(effect, Effect::Call { site, operation: true, .. } if *site == call.site)
                || *owner != call.owner
                || observations.sources.contains_key(&Port::Operation(point))
                || observations.forwarded.contains_key(&Port::Operation(point))
            {
                return Err(invalid(span));
            }
            let port = Port::Operation(point);
            let node = graph.nodes.get(&port).ok_or_else(|| invalid(span))?;
            let [input] = node.incoming.as_slice() else {
                return Err(invalid(span));
            };
            let edge = Edge::new(call.input, port, Route::Next);
            let from = graph.nodes.get(&call.input);
            let site = from
                .map(|node| node.site)
                .or_else(|| graph.external.get(&call.input).copied());
            if node.site != self.scalar_port_site(port, call.owner, span)?
                || input.source != self.scalar_port_site(call.input, call.owner, span)?
                || site != Some(input.source)
                || input.reached != from.is_some()
                || graph.edges.get(input.edge) != Some(&(Family::Invocation, edge))
            {
                return Err(invalid(span));
            }
        }
        if observations.calls.replace(calls).is_some() {
            return Err(invalid(span));
        }
        Ok((observations, parts))
    }
}

#[cfg(test)]
mod tests;
