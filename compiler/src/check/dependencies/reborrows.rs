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
pub(crate) struct Reborrow {
    pub(crate) owner: usize,
    pub(crate) parent: hir::PointId,
    pub(crate) site: Option<hir::ReborrowId>,
    pub(crate) mode: hir::ReferenceMode,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn reborrow_operation(
        &mut self,
        id: hir::PointId,
        parent: hir::PointId,
        value: &hir::Expr,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof reborrow-operation budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof reborrow-operation identity mismatch", span);
        if !self
            .flow
            .spend(self.reborrow_ops.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || !self.points.get(parent).is_some_and(|child| {
                child.parent == Some(id)
                    && child.owner == self.owner
                    && child.block == point.block
                    && child.complete
                    && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
        {
            return Err(invalid());
        }
        let site = if value.ty == hir::Type::Never {
            None
        } else {
            let hir::ExprKind::Reborrow {
                site,
                value: input,
                fields,
            } = &value.kind
            else {
                return Err(invalid());
            };
            let hir::Type::Exclusive(ty) = &value.ty else {
                return Err(invalid());
            };
            if !matches!(
                ty.as_ref(),
                hir::Type::Bool | hir::Type::Int { .. } | hir::Type::Float { .. }
            ) || input.ty != value.ty
                || !fields.is_empty()
                || *site >= self.reborrows
            {
                return Err(invalid());
            }
            Some(*site)
        };
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(parent), Route::Next)];
        if site.is_some() {
            edges.extend([
                Edge::new(Port::Normal(parent), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]);
        }
        let reborrow = Reborrow {
            owner: self.owner,
            parent,
            site,
            mode: hir::ReferenceMode::Exclusive,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.reborrow_ops.get(&id) {
            return if *prior == reborrow {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(reborrow.edges.len()) {
            return Err(budget());
        }
        self.reborrow_edges += reborrow.edges.len();
        self.reborrow_ops.insert(id, reborrow);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
