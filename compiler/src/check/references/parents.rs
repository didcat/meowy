use super::{Checker, Result};
use crate::ast::{self, ExprKind};
use crate::hir::{self, Type};

pub(crate) type Temporary = (hir::LocalId, hir::StatementId);

impl Checker {
    pub(crate) fn projected_parent(
        &mut self,
        root: &ast::Expr,
    ) -> Result<(hir::PointId, Option<Temporary>, hir::Expr)> {
        match &root.kind {
            ExprKind::Index { .. } => self
                .borrowed_point(root, root.span)
                .map(|(point, value)| (point, None, value)),
            ExprKind::Unary { op, value } if op == "*" => self
                .expr_point(value, None)
                .map(|(point, value)| (point, None, value)),
            _ => {
                let (point, value) = self.expr_point(root, None)?;
                if !matches!(value.ty, Type::Reference(_) | Type::Exclusive(_))
                    && self.address(root).is_err()
                {
                    let value = self.temporary_borrow(value, root.span)?;
                    let temporary = match &value.kind {
                        hir::ExprKind::TemporaryBorrow { id, statement, .. } => {
                            Some((*id, *statement))
                        }
                        _ => None,
                    };
                    Ok((point, temporary, value))
                } else {
                    Ok((point, None, value))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
