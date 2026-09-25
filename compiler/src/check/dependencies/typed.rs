use super::{
    PointKind,
    edges::{Edge, Port, Route},
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Predicate,
    Ascription,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Typed {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) kind: Kind,
    pub(crate) normal: bool,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn typed_operation(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        predicate: bool,
        value: &hir::Expr,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof typed-operation budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof typed-operation identity mismatch", span);
        if !self
            .flow
            .spend(self.typed_ops.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
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
        {
            return Err(invalid());
        }
        let kind = if predicate {
            Kind::Predicate
        } else {
            Kind::Ascription
        };
        let normal = value.ty != hir::Type::Never;
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(input), Route::Next)];
        if normal {
            edges.extend([
                Edge::new(Port::Normal(input), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]);
        }
        let op = Typed {
            owner: self.owner,
            input,
            kind,
            normal,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.typed_ops.get(&id) {
            return if *prior == op { Ok(()) } else { Err(invalid()) };
        }
        if !self.edge_room(op.edges.len()) {
            return Err(budget());
        }
        self.typed_edges += op.edges.len();
        self.typed_ops.insert(id, op);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
