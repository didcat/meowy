use super::{
    PointKind, SequenceSource,
    edges::{Edge, Port, Route},
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Dispatch {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) local: hir::LocalId,
    pub(crate) block: hir::BlockId,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn dispatch_operation(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        local: hir::LocalId,
        body: &hir::Block,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof dispatch-operation budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof dispatch-operation identity mismatch", span);
        let work = [
            self.dispatch_ops.len(),
            self.dispatch_ops.len(),
            self.bodies.len(),
            self.sequences.len(),
            self.endpoints.len(),
            self.proofs.dispatches.len(),
            self.proofs.receivers.len(),
        ]
        .into_iter()
        .fold(6, |work, len| {
            work + len.checked_ilog2().unwrap_or(0) as usize
        });
        if !self.flow.spend(work) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || !self.points.get(input).is_some_and(|child| {
                child.parent == Some(id)
                    && child.owner == self.owner
                    && child.block == point.block
                    && child.complete
                    && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
            || !self
                .bodies
                .get(&body.id)
                .is_some_and(|body| body.owner == self.owner && body.parent == Some(id))
            || !self.proofs.dispatches.contains(&body.id)
            || !self.proofs.receivers.contains(&local)
        {
            return Err(invalid());
        }
        let key = SequenceSource::Block(body.id);
        let sequence = self.sequences.get(&key).ok_or_else(invalid)?;
        if sequence.owner != self.owner
            || sequence.items.first() != Some(&None)
            || !self.endpoints.contains_key(&key)
        {
            return Err(invalid());
        }
        if let Some(Some(next)) = sequence.items.get(1)
            && !self.points.get(*next).is_some_and(|point| {
                point.owner == self.owner
                    && point.block == Some(body.id)
                    && point.kind == PointKind::Stmt
                    && point.complete
            })
        {
            return Err(invalid());
        }
        if sequence.items.len() == 1 && body.stmts.len() != 1 {
            return Err(invalid());
        }
        let Some(hir::Stmt::Bind { id: stored, value }) = body.stmts.first() else {
            return Err(invalid());
        };
        if *stored != local {
            return Err(invalid());
        }
        let ty = self.locals.get(local).ok_or_else(invalid)?;
        let size = crate::borrow_contract::type_weight(ty, &mut self.flow, span)?;
        let count = crate::borrow_contract::type_weight(&value.ty, &mut self.flow, span)?;
        if !self.flow.spend(size.saturating_add(count)) {
            return Err(budget());
        }
        if ty != &value.ty {
            return Err(invalid());
        }
        let mut edges = vec![
            Edge::new(Port::Entry(id), Port::BlockEntry(body.id), Route::Next),
            Edge::new(Port::BlockEntry(body.id), Port::Entry(input), Route::Next),
        ];
        if value.ty != hir::Type::Never {
            edges.push(Edge::new(
                Port::Normal(input),
                Port::Operation(id),
                Route::Next,
            ));
            let next = match sequence.items.get(1) {
                Some(Some(next)) => Some(Port::Entry(*next)),
                None => Some(Port::BlockNormal(body.id)),
                Some(None) => None,
            };
            if let Some(next) = next {
                edges.push(Edge::new(Port::Operation(id), next, Route::Next));
            }
            if body.ty != hir::Type::Never {
                edges.push(Edge::new(
                    Port::BlockResult(body.id),
                    Port::Normal(id),
                    Route::Result,
                ));
            }
        }
        let op = Dispatch {
            owner: self.owner,
            input,
            local,
            block: body.id,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.dispatch_ops.get(&id) {
            return if *prior == op { Ok(()) } else { Err(invalid()) };
        }
        if !self.edge_room(op.edges.len()) {
            return Err(budget());
        }
        self.dispatch_edges += op.edges.len();
        self.dispatch_ops.insert(id, op);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
