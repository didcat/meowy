use crate::ast::{self, ExprKind};
use crate::check::{
    Checker, Result, Value,
    type_values::{MAX_DEPTH, MAX_WORK, Work},
};
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
            ExprKind::Binary { op, left, right } if matches!(op.as_str(), "&&" | "||") => {
                let left = self.boolean_form(left, depth + 1, count)?;
                let right = self.boolean_form(right, depth + 1, count)?;
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
