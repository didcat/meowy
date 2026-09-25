use super::*;
use crate::check::dependencies::CoercionKind;

impl Checker {
    pub(crate) fn composed_fallback_point(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<(hir::PointId, CoercionKind, hir::Expr)> {
        let (input, value) = self.expression_point(expr, expected)?;
        let stopped = value.ty == Type::Never;
        let (changed, value) = if matches!(value.ty, Type::Record { .. }) {
            (false, value)
        } else if let Some(expected) = expected
            && expected.accepts(&value.ty)
        {
            Self::coercion(value, expected.clone())
        } else {
            (false, value)
        };
        let kind = if stopped {
            CoercionKind::Stopped
        } else if changed {
            CoercionKind::Convert
        } else {
            CoercionKind::Forward
        };
        Ok((input, kind, value))
    }
}

#[cfg(test)]
mod tests;
