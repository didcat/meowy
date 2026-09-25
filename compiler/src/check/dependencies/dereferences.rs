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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Deref {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) mode: Option<hir::ReferenceMode>,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn deref_operation(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        value: &hir::Expr,
        span: Span,
    ) -> Result<()> {
        let budget =
            || Diagnostic::unsupported("proof dereference-operation budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof dereference-operation identity mismatch", span);
        if !self
            .flow
            .spend(self.derefs.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
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
        let mode = match value.ty {
            hir::Type::Reference(_) => Some(hir::ReferenceMode::Shared),
            hir::Type::Exclusive(_) => Some(hir::ReferenceMode::Exclusive),
            hir::Type::Never => None,
            _ => return Err(invalid()),
        };
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(input), Route::Next)];
        if mode.is_some() {
            edges.push(Edge::new(
                Port::Normal(input),
                Port::Operation(id),
                Route::Next,
            ));
            if value.ty.pointee() != Some(&hir::Type::Never) {
                edges.push(Edge::new(
                    Port::Operation(id),
                    Port::Normal(id),
                    Route::Next,
                ));
            }
        }
        let deref = Deref {
            owner: self.owner,
            input,
            mode,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.derefs.get(&id) {
            return if *prior == deref {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(deref.edges.len()) {
            return Err(budget());
        }
        self.deref_edges += deref.edges.len();
        self.derefs.insert(id, deref);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
