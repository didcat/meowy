use super::*;
use crate::check::dependencies::SequenceSource;

mod forward;
mod pending;
mod type_aliases;

impl Checker {
    pub(crate) fn function_declaration_endpoint(
        &mut self,
        id: PointId,
        function: crate::hir::FunctionId,
        span: Span,
    ) -> Result<()> {
        let budget = || {
            Diagnostic::unsupported("proof function declaration endpoint budget exhausted", span)
        };
        let invalid = || {
            Diagnostic::unsupported(
                "proof function declaration endpoint identity mismatch",
                span,
            )
        };
        if !self.flow.spend(
            self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + self.endpoints.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 4,
        ) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let owner = function.checked_add(1).ok_or_else(invalid)?;
        let item = self
            .functions
            .get(function)
            .and_then(Option::as_ref)
            .ok_or_else(invalid)?;
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || item.id != function
            || owner == self.owner
            || !self
                .bodies
                .get(&item.body.id)
                .is_some_and(|body| body.owner == owner && body.parent.is_none())
        {
            return Err(invalid());
        }
        self.publish_declaration_endpoint(id, span, "function declaration")
    }

    pub(super) fn publish_declaration_endpoint(
        &mut self,
        id: PointId,
        span: Span,
        kind: &'static str,
    ) -> Result<()> {
        let key = SequenceSource::Stmt(id);
        let edges = [Edge::new(Port::Entry(id), Port::Normal(id), Route::Next)];
        if let Some(prior) = self.endpoints.get(&key) {
            return if *prior == edges {
                Ok(())
            } else {
                Err(Diagnostic::unsupported(
                    format!("proof {kind} endpoint identity mismatch"),
                    span,
                ))
            };
        }
        if !self.edge_room(edges.len()) {
            return Err(Diagnostic::unsupported(
                format!("proof {kind} endpoint budget exhausted"),
                span,
            ));
        }
        self.endpoint_edges += edges.len();
        self.endpoints.insert(key, edges.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod exports;
