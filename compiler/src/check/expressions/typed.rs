use super::*;

impl Checker {
    pub(crate) fn typed_point(
        &mut self,
        value: &ast::Expr,
        ty: &ast::TypeExpr,
    ) -> Result<(hir::PointId, hir::Expr, Type)> {
        let (input, value) = self.expr_point(value, None)?;
        let ty = self.construct_type(ty)?;
        Ok((input, value, ty))
    }
}

#[cfg(test)]
mod tests;
