use super::*;
use crate::check::dependencies::{CoercionKind, PointKind};

impl Checker {
    pub(crate) fn binary_operand_point(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
        equality: bool,
    ) -> Result<(hir::PointId, hir::Expr)> {
        if !self.required
            && equality
            && let Some(expected @ Type::List { .. }) = expected
        {
            self.operand_root(expr, expected)
        } else {
            self.operand_point(expr, expected)
        }
    }

    pub(crate) fn operand_constructor(mut expr: &ast::Expr) -> bool {
        while let ExprKind::Group(child) = &expr.kind {
            expr = child;
        }
        matches!(
            expr.kind,
            ExprKind::Block(_) | ExprKind::DispatchBlock { .. }
        )
    }

    pub(crate) fn operand_point(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<(hir::PointId, hir::Expr)> {
        let Some(expected) = expected.filter(|ty| {
            !self.required
                && matches!(
                    ty,
                    Type::Null
                        | Type::Bool
                        | Type::Int { .. }
                        | Type::Float { .. }
                        | Type::String
                        | Type::Reference(_)
                )
                && Self::operand_constructor(expr)
        }) else {
            return self.expression_point(expr, expected);
        };
        self.operand_root(expr, expected)
    }

    pub(crate) fn operand_root(
        &mut self,
        expr: &ast::Expr,
        expected: &Type,
    ) -> Result<(hir::PointId, hir::Expr)> {
        self.with_continuation(expr.span, "expression", |checker| {
            checker.with_point_id(PointKind::expression(expr), expr.span, |checker| {
                checker.operand_value(expr, expected)
            })
        })
    }

    pub(crate) fn primary_emission_point(
        &mut self,
        expr: &ast::Expr,
        expected: &Type,
    ) -> Result<(hir::PointId, hir::Expr)> {
        let constructor = Self::operand_constructor(expr) || matches!(expected, Type::List { .. });
        self.operand_coercion(expr, expected, |checker| {
            if constructor {
                checker.operand_value(expr, expected)
            } else {
                checker.expression_value(expr, Some(expected))
            }
        })
    }

    pub(crate) fn operand_coercion(
        &mut self,
        expr: &ast::Expr,
        expected: &Type,
        check: impl FnOnce(&mut Self) -> Result<hir::Expr>,
    ) -> Result<(hir::PointId, hir::Expr)> {
        self.with_continuation(expr.span, "expression", |checker| {
            checker.with_point_id(PointKind::Expr, expr.span, |checker| {
                let (input, value) =
                    checker.with_point_id(PointKind::expression(expr), expr.span, check)?;
                if matches!(value.ty, Type::Record { .. }) {
                    checker.coercion_operation(
                        checker.point.expect("operand context"),
                        input,
                        CoercionKind::Forward,
                        expr.span,
                    )?;
                    return Ok(value);
                }
                checker.coerced_value(value, Some(expected), Some(input), expr.span)
            })
        })
    }

    pub(crate) fn operand_value(&mut self, expr: &ast::Expr, expected: &Type) -> Result<hir::Expr> {
        if !matches!(
            expr.kind,
            ExprKind::Group(_) | ExprKind::Block(_) | ExprKind::DispatchBlock { .. }
        ) {
            return self.expression_value(expr, Some(expected));
        }
        self.charge_integer(expr)?;
        let value = match &expr.kind {
            ExprKind::Group(child) => {
                let (input, value) = self.operand_coercion(child, expected, |checker| {
                    checker.operand_value(child, expected)
                })?;
                if let Some(point) = self.point {
                    self.group_region(point, input, expr.span)?;
                }
                value
            }
            ExprKind::Block(block) | ExprKind::DispatchBlock { block, .. } => {
                let receiver = if let ExprKind::DispatchBlock { value, .. } = &expr.kind {
                    let (input, value) = self.expr_point(value, None)?;
                    if value.ty.has_exclusive() {
                        return Err(Diagnostic::unsupported(
                            "exclusive dispatch receivers",
                            expr.span,
                        ));
                    }
                    Some((input, value))
                } else {
                    None
                };
                let input = receiver.as_ref().map(|(input, _)| *input);
                let prefix = self.block_prefix(
                    block,
                    Some(expected.clone()),
                    receiver.map(|(_, value)| value),
                    false,
                )?;
                self.frames.last_mut().expect("operand frame").primary = true;
                let (local, body) = self.block_contents(block, prefix)?;
                if let Some(point) = self.point {
                    if let Some(input) = input {
                        self.dispatch_operation(
                            point,
                            input,
                            local.expect("operand receiver"),
                            &body,
                            expr.span,
                        )?;
                    } else {
                        self.block_result(point, body.id, expr.span)?;
                    }
                }
                hir::Expr {
                    ty: body.ty.clone(),
                    kind: hir::ExprKind::Block(body),
                    span: expr.span,
                }
            }
            _ => unreachable!("operand constructor"),
        };
        if value.ty == Type::Bool {
            self.guard(&value);
        }
        if value.ty == Type::Never {
            self.reach = FALSE;
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests;
