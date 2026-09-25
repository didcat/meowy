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
    pub(crate) parent_mode: Option<hir::ReferenceMode>,
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
        mode: hir::ReferenceMode,
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
        let (site, parent_mode) = if value.ty == hir::Type::Never {
            (None, None)
        } else {
            let hir::ExprKind::Reborrow {
                site,
                value: input,
                fields,
            } = &value.kind
            else {
                return Err(invalid());
            };
            if !fields.is_empty() || *site >= self.reborrows {
                return Err(invalid());
            }
            let (parent_mode, pointee) = match &input.ty {
                hir::Type::Reference(ty) => (hir::ReferenceMode::Shared, ty.as_ref()),
                hir::Type::Exclusive(ty) => (hir::ReferenceMode::Exclusive, ty.as_ref()),
                _ => return Err(invalid()),
            };
            match (mode, &value.ty) {
                (hir::ReferenceMode::Exclusive, hir::Type::Exclusive(ty)) => {
                    if parent_mode != mode
                        || !matches!(
                            ty.as_ref(),
                            hir::Type::Bool | hir::Type::Int { .. } | hir::Type::Float { .. }
                        )
                        || pointee != ty.as_ref()
                    {
                        return Err(invalid());
                    }
                }
                (hir::ReferenceMode::Shared, hir::Type::Reference(ty)) => {
                    let size = crate::borrow_contract::type_weight(ty, &mut self.flow, span)?;
                    let count = crate::borrow_contract::type_weight(pointee, &mut self.flow, span)?;
                    if !self
                        .flow
                        .spend(size.saturating_mul(2).saturating_add(count))
                    {
                        return Err(budget());
                    }
                    if ty.has_exclusive() || pointee != ty.as_ref() {
                        return Err(invalid());
                    }
                }
                _ => return Err(invalid()),
            }
            (Some(*site), Some(parent_mode))
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
            parent_mode,
            site,
            mode,
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

#[cfg(test)]
mod shared;
