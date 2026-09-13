use crate::ast::{self, ExprKind};
use crate::check::{
    Checker, Result, Value,
    type_values::{MAX_DEPTH, MAX_WORK, Work},
};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn boolean_form(
        &mut self,
        expr: &ast::Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<Type> {
        *count += 1;
        if depth >= MAX_DEPTH || *count > MAX_WORK || !self.flow.spend(1) {
            return Err(Work::budget(expr.span));
        }
        match &expr.kind {
            ExprKind::Name(name) => match self.required_value(name, expr.span)? {
                Value::Local { ty, .. }
                | Value::Static { ty, .. }
                | Value::FileModule { ty, .. } => Ok(ty),
                Value::Constant(value) => Ok(Self::constant_expr(value, expr.span).ty),
                _ => Err(Self::error(
                    "E222",
                    "required boolean operand is not a scalar value",
                    expr.span,
                )),
            },
            ExprKind::Field { .. } => self.required_path(expr).map(|(_, ty, _)| ty),
            ExprKind::Group(value) => self.boolean_form(value, depth + 1, count),
            ExprKind::Unary { op, value } if op == "!" => {
                let ty = self.boolean_form(value, depth + 1, count)?;
                if Self::primary_type(&ty) != Type::Bool {
                    return Err(Self::error(
                        "E222",
                        format!("operator `!` is not defined for {ty:?}"),
                        expr.span,
                    ));
                }
                Ok(Type::Bool)
            }
            ExprKind::Binary { op, left, right }
                if matches!(op.as_str(), "&&" | "||" | "==" | "!=") =>
            {
                let left = self.boolean_form(left, depth + 1, count)?;
                let right = self.boolean_form(right, depth + 1, count)?;
                if matches!(op.as_str(), "==" | "!=")
                    && (matches!((&left, &right), (Type::Record { .. }, Type::Record { .. }))
                        || Self::primary_type(&left) != Type::Bool
                            && Self::primary_type(&right) != Type::Bool)
                {
                    return Err(Diagnostic::unsupported(
                        "required comparisons outside boolean operands",
                        expr.span,
                    ));
                }
                if Self::primary_type(&left) != Type::Bool
                    || Self::primary_type(&right) != Type::Bool
                {
                    return Err(Self::error(
                        "E222",
                        format!(
                            "operator `{op}` requires boolean operands, found {left:?} and {right:?}"
                        ),
                        expr.span,
                    ));
                }
                Ok(Type::Bool)
            }
            ExprKind::Int(_) => Ok(Type::Int {
                bits: 32,
                signed: true,
            }),
            ExprKind::Float(_) => Ok(Type::Float { bits: 64 }),
            ExprKind::String(_) => Ok(Type::String),
            _ => Err(self.type_unavailable(expr)?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Span};
    use crate::check::Constant;

    pub(crate) fn tree(leaves: usize) -> Expr {
        Expr {
            span: Span::new(0, 1),
            kind: if leaves == 1 {
                ExprKind::Name("false".into())
            } else {
                ExprKind::Binary {
                    op: "||".into(),
                    left: Box::new(tree(leaves / 2)),
                    right: Box::new(tree(leaves - leaves / 2)),
                }
            },
        }
    }

    #[test]
    pub(crate) fn required_boolean_forms_bound_skipped_trees_without_charging_evaluation() {
        for leaves in [2047, 2048] {
            let mut checker = Checker::new();
            checker.type_work = Some(Work::default());
            let expr = Expr {
                span: Span::new(0, 1),
                kind: ExprKind::Binary {
                    op: "&&".into(),
                    left: Box::new(tree(1)),
                    right: Box::new(tree(leaves)),
                },
            };
            let result = checker.type_scalar(&expr, None);
            if leaves == 2047 {
                assert!(matches!(
                    result.unwrap(),
                    Value::Static {
                        value: Constant::Bool(false),
                        ..
                    }
                ));
                assert_eq!(checker.type_work.as_ref().unwrap().visits, 2);
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
                assert_eq!(checker.type_work.as_ref().unwrap().visits, 0);
            }
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
        }
    }
}
