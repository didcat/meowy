use super::*;
use crate::check::dependencies::CoercionKind;

impl Checker {
    pub(crate) fn expected_plan(
        value: hir::Expr,
        expected: &Type,
        span: Span,
    ) -> Result<(bool, CoercionKind, hir::Expr)> {
        if value.ty == Type::Never {
            return Ok((false, CoercionKind::Stopped, value));
        }
        if value.ty != *expected {
            if expected.accepts(&value.ty) {
                return Ok((
                    false,
                    CoercionKind::Convert,
                    Self::coerce(value, expected.clone()),
                ));
            }
            if let Type::Record { primary, .. } = &value.ty
                && expected.accepts(primary)
                && !matches!(expected, Type::Record { .. })
            {
                let ty = *primary.clone();
                let stopped = ty == Type::Never;
                let (changed, value) = Self::coercion(
                    hir::Expr {
                        ty,
                        span,
                        kind: hir::ExprKind::Primary(Box::new(value)),
                    },
                    expected.clone(),
                );
                let kind = if stopped {
                    CoercionKind::Stopped
                } else if changed {
                    CoercionKind::Convert
                } else {
                    CoercionKind::Forward
                };
                return Ok((true, kind, value));
            }
            return Err(Self::error(
                "E207",
                format!("expected {expected:?}, found {:?}", value.ty),
                span,
            ));
        }
        Ok((false, CoercionKind::Forward, value))
    }
}

#[cfg(test)]
mod tests;
