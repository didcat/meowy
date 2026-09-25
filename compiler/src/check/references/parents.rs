use super::{Checker, Result};
use crate::ast::{self, ExprKind};
use crate::hir::{self, Type};

impl Checker {
    pub(crate) fn projected_parent(
        &mut self,
        root: &ast::Expr,
    ) -> Result<(hir::PointId, hir::Expr)> {
        match &root.kind {
            ExprKind::Index { .. } => self.borrowed_point(root, root.span),
            ExprKind::Unary { op, value } if op == "*" => self.expr_point(value, None),
            _ => {
                let (point, value) = self.expr_point(root, None)?;
                let value = if !matches!(value.ty, Type::Reference(_) | Type::Exclusive(_))
                    && self.address(root).is_err()
                {
                    self.temporary_borrow(value, root.span)?
                } else {
                    value
                };
                Ok((point, value))
            }
        }
    }
}

#[cfg(test)]
mod tests;
