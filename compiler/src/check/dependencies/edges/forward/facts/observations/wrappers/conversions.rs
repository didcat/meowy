use super::*;

impl Checker {
    pub(super) fn scalar_conversion_observation<'a>(
        &mut self,
        reports: &Reports,
        graph: &graph::Graph<'_>,
        source: (PointId, &'a (usize, Effect)),
        observations: &mut Observations<'a, '_>,
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        let (point, source) = source;
        let (owner, Effect::Coercion(op)) = source else {
            return Err(invalid(span));
        };
        if !self.flow.spend(6) {
            return Err(budget(span));
        }
        if op.scalar.is_none()
            || self
                .forward_coercion_input(reports, point, *owner, span)?
                .is_some()
            || op.projected
            || graph
                .nodes
                .contains_key(&Port::Projection { point, step: 0 })
        {
            return Err(invalid(span));
        }
        for (port, from, reached) in [
            (Port::Operation(point), Port::Normal(op.input), op.operation),
            (Port::Normal(point), Port::Operation(point), op.result),
        ] {
            if !self.flow.spend(4) {
                return Err(budget(span));
            }
            if reached != graph.nodes.contains_key(&port) {
                return Err(invalid(span));
            }
            if !reached {
                continue;
            }
            self.coercion_effect_stage(reports, *owner, port, span)?
                .ok_or_else(|| invalid(span))?;
            self.scalar_scope_edges(
                graph,
                *owner,
                port,
                Family::Coercion,
                &[Edge::new(from, port, Route::Next)],
                span,
            )?;
            if observations.sources.contains_key(&port)
                || observations.forwarded.contains_key(&port)
                || observations.logical.contains_key(&port)
            {
                return Err(invalid(span));
            }
            *parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            observations.sources.insert(port, source);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
