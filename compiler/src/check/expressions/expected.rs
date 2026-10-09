use super::*;
use crate::check::dependencies::CoercionKind;

impl Checker {
    pub(crate) fn coerced_value(
        &mut self,
        mut value: hir::Expr,
        expected: Option<&Type>,
        input: Option<hir::PointId>,
        span: Span,
    ) -> Result<hir::Expr> {
        let shared = matches!(expected, Some(Type::Reference(_)));
        if value.ty == Type::Never {
            self.reach = FALSE;
            if let Some(source) = input {
                let point = self.point.expect("expected context");
                if shared {
                    self.reborrow_operation(
                        point,
                        source,
                        hir::ReferenceMode::Shared,
                        &value,
                        span,
                    )?;
                } else {
                    self.coercion_operation(point, source, CoercionKind::Stopped, span)?;
                }
            }
            return Ok(value);
        }
        if let (Some(Type::Reference(target)), Type::Exclusive(source)) = (expected, &value.ty)
            && target == source
        {
            let site = self.reborrows;
            self.reborrows += 1;
            value = hir::Expr {
                kind: hir::ExprKind::Reborrow {
                    site,
                    value: Box::new(value),
                    fields: Vec::new(),
                },
                ty: expected.unwrap().clone(),
                span,
            };
            self.reborrow_operation(
                self.point.expect("shared context"),
                input.expect("shared conversion source"),
                hir::ReferenceMode::Shared,
                &value,
                span,
            )?;
            return Ok(value);
        }
        let Some(expected) = expected else {
            return Ok(value);
        };
        let shape = Coercion::primary_source(&value.ty);
        let (primary, kind, value) = Self::expected_plan(value, expected, span)?;
        if let Some(source) = input {
            let point = self.point.expect("expected context");
            if shared && !primary && kind == CoercionKind::Forward {
                self.region_edges(point, source, span)?;
            } else if !self.required {
                self.coercion_stages(
                    point,
                    source,
                    kind,
                    primary,
                    if primary { shape } else { None },
                    span,
                )?;
            }
        }
        Ok(value)
    }

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
