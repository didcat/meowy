use super::*;

impl Checker {
    pub(crate) fn dispatch_point(
        &mut self,
        receiver: &ast::Expr,
        block: &ast::Block,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(hir::PointId, hir::LocalId, hir::Block)> {
        let (input, value) = self.expr_point(receiver, None)?;
        if value.ty.has_exclusive() {
            return Err(Diagnostic::unsupported(
                "exclusive dispatch receivers",
                span,
            ));
        }
        let (local, block) = self.block_parts(block, expected.cloned(), Some(value), false)?;
        Ok((input, local.expect("dispatch receiver"), block))
    }
}

#[cfg(test)]
mod tests;
