use super::*;
use crate::check::dependencies::bodies::completion::Shape;

impl Checker {
    pub(in super::super) fn scalar_call_result_observations<'a, 'd>(
        &mut self,
        reports: &'a Reports,
        graph: &graph::Graph<'_>,
        mut observations: Observations<'a, 'd>,
        span: Span,
        parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        for (&point, source) in &reports.effects {
            let (
                owner,
                Effect::Call {
                    shape,
                    control,
                    operation,
                    result,
                    ..
                },
            ) = source
            else {
                continue;
            };
            self.validate_call_report(reports, point, *owner, &source.1, span)?;
            if !self.flow.spend(8) {
                return Err(budget(span));
            }
            let port = Port::Normal(point);
            let from = Port::Operation(point);
            if *operation != graph.nodes.contains_key(&from)
                || *result != graph.nodes.contains_key(&port)
            {
                return Err(invalid(span));
            }
            if !result || !matches!(shape, Shape::Scalar(_)) {
                continue;
            }
            let node = &graph.nodes[&port];
            let [input] = node.incoming.as_slice() else {
                return Err(invalid(span));
            };
            let site = graph
                .nodes
                .get(&from)
                .map(|node| node.site)
                .or_else(|| graph.external.get(&from).copied());
            if node.site != self.scalar_port_site(port, *owner, span)?
                || input.source != self.scalar_port_site(from, *owner, span)?
                || site != Some(input.source)
                || input.reached != *operation
                || graph.edges.get(input.edge)
                    != Some(&(Family::Invocation, Edge::new(from, port, Route::Returned)))
                || observations.sources.contains_key(&port)
                || observations.forwarded.contains_key(&port)
                || observations.logical.contains_key(&port)
                || observations.call_controls.contains_key(&port)
            {
                return Err(invalid(span));
            }
            let control = super::super::super::controls::dependency(
                point,
                *owner,
                *control,
                &self.points,
                &self.branch_inputs,
                &mut self.flow,
            )
            .map_err(|error| error.diagnostic("scalar-call", span))?;
            parts = parts.checked_sub(2).ok_or_else(|| budget(span))?;
            observations.sources.insert(port, source);
            observations.call_controls.insert(port, control);
        }
        Ok((observations, parts))
    }
}

#[cfg(test)]
mod tests;
