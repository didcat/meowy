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
pub(crate) struct Cell {
    pub(crate) local: hir::LocalId,
    pub(crate) statement: hir::StatementId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Temporary {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) cell: Option<Cell>,
    pub(crate) control: bool,
    pub(crate) span: Span,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn temporary_operation(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        value: &hir::Expr,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof temporary-borrow budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof temporary-borrow identity mismatch", span);
        if !self.flow.spend(
            self.temporary_borrows.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                + 4,
        ) {
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
        let cell = if value.ty == hir::Type::Never {
            None
        } else {
            let hir::ExprKind::TemporaryBorrow {
                id: local,
                statement,
                value: source,
            } = &value.kind
            else {
                return Err(invalid());
            };
            let hir::Type::Reference(target) = &value.ty else {
                return Err(invalid());
            };
            let stored = self.locals.get(*local).ok_or_else(invalid)?;
            if self.proofs.temporaries.get(local) != Some(statement)
                || source.ty == hir::Type::Never
            {
                return Err(invalid());
            }
            let size = crate::borrow_contract::type_weight(stored, &mut self.flow, span)?;
            let count = crate::borrow_contract::type_weight(&source.ty, &mut self.flow, span)?;
            let width = crate::borrow_contract::type_weight(target, &mut self.flow, span)?;
            if !self.flow.spend(
                size.saturating_mul(2)
                    .saturating_add(count)
                    .saturating_add(width),
            ) {
                return Err(budget());
            }
            if stored != &source.ty || stored != target.as_ref() || target.has_exclusive() {
                return Err(invalid());
            }
            Some(Cell {
                local: *local,
                statement: *statement,
            })
        };
        let mut edges = vec![Edge::new(Port::Entry(id), Port::Entry(input), Route::Next)];
        if cell.is_some() {
            edges.extend([
                Edge::new(Port::Normal(input), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]);
        }
        let temporary = Temporary {
            owner: self.owner,
            input,
            cell,
            control: self.control,
            span,
            edges,
        };
        if let Some(prior) = self.temporary_borrows.get(&id) {
            return if *prior == temporary {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(temporary.edges.len()) {
            return Err(budget());
        }
        self.temporary_borrow_edges += temporary.edges.len();
        self.temporary_borrows.insert(id, temporary);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
