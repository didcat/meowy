use crate::ast::{self, ExprKind, Span};
use crate::check::{Checker, Constant, Result, Value, inputs::Input};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn charge_integer(&mut self, expr: &ast::Expr) -> Result<()> {
        if !self.required {
            return Ok(());
        }
        let Some(work) = &mut self.type_work else {
            return Ok(());
        };
        let steps = match &expr.kind {
            ExprKind::Int(_) | ExprKind::Name(_) | ExprKind::Field { .. } => 1,
            ExprKind::Unary { op, value }
                if op == "-" && matches!(value.kind, ExprKind::Int(_)) =>
            {
                2
            }
            ExprKind::Unary { op, .. } if matches!(op.as_str(), "-" | "~") => 1,
            ExprKind::Binary { op, .. }
                if matches!(op.as_str(), "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^") =>
            {
                1
            }
            _ => 0,
        };
        work.logical.charge(0, 0)?;
        for _ in 0..steps {
            work.logical.charge(1, 0)?;
        }
        Ok(())
    }

    pub(crate) fn required_value(&mut self, name: &str, span: Span) -> Result<Value> {
        let saved = std::mem::replace(&mut self.required, true);
        let result = self.value(name, span);
        self.required = saved;
        result
    }

    pub(crate) fn required_hint(&mut self, expr: &ast::Expr) -> Option<Type> {
        let saved = std::mem::replace(&mut self.required, true);
        let ty = self.hint(expr);
        self.required = saved;
        ty
    }

    pub(crate) fn required_primary(&self, id: usize, span: Span) -> Result<Input> {
        self.module_integer(id).cloned().ok_or_else(|| {
            Self::error(
                "E211",
                "primary initializer is unavailable during required type evaluation",
                span,
            )
        })
    }

    pub(crate) fn scalar_input(&mut self, expr: &ast::Expr) -> Result<()> {
        self.type_work.as_mut().unwrap().enter(expr.span)?;
        let result = (|| match &expr.kind {
            ExprKind::Int(_) => Ok(()),
            ExprKind::Name(name) => match self.required_value(name, expr.span)? {
                Value::Static {
                    value: Constant::Int(_),
                    ..
                }
                | Value::Constant(Constant::Int(_)) => Ok(()),
                Value::Local { id, .. } if self.inputs.contains_key(&id) => self
                    .type_work
                    .as_mut()
                    .unwrap()
                    .input(&self.inputs[&id], expr.span),
                Value::FileModule { id, ty }
                    if matches!(Self::primary_type(&ty), Type::Int { .. }) =>
                {
                    let input = self.required_primary(id, expr.span)?;
                    self.type_work.as_mut().unwrap().input(&input, expr.span)
                }
                Value::Local { constant: None, .. } => Err(Self::error(
                    "E211",
                    "runtime input is unavailable during required type evaluation",
                    expr.span,
                )),
                Value::Local {
                    constant: Some(_), ..
                } => Err(Diagnostic::unsupported(
                    "runtime initializer eligibility for required type evaluation",
                    expr.span,
                )),
                _ => Err(Diagnostic::unsupported(
                    "non-integer computed scalar inputs",
                    expr.span,
                )),
            },
            ExprKind::Field { .. } => {
                let (_, input) = self.required_field(expr)?;
                self.type_work.as_mut().unwrap().input(&input, expr.span)
            }
            ExprKind::Group(value) => self.scalar_input(value),
            ExprKind::Unary { op, value } if matches!(op.as_str(), "-" | "~") => {
                self.scalar_input(value)
            }
            ExprKind::Binary { op, left, right }
                if matches!(op.as_str(), "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^") =>
            {
                self.scalar_input(left)?;
                self.scalar_input(right)
            }
            _ => Err(self.type_unavailable(expr)?),
        })();
        self.type_work.as_mut().unwrap().depth -= 1;
        result
    }

    pub(crate) fn boolean_scalar(&mut self, expr: &ast::Expr) -> bool {
        let mut form = expr;
        while let ExprKind::Group(value) = &form.kind {
            form = value;
        }
        matches!(&form.kind, ExprKind::Unary { op, .. } if op == "!")
            || self
                .required_hint(expr)
                .is_some_and(|ty| Self::primary_type(&ty) == Type::Bool)
    }

    pub(crate) fn type_scalar(
        &mut self,
        expr: &ast::Expr,
        annotation: Option<&ast::TypeExpr>,
    ) -> Result<Value> {
        if self.boolean_scalar(expr) {
            return self.type_boolean(expr, annotation);
        }
        if self.integer_blocks(expr)? {
            let expected = annotation
                .map(|ty| self.source_type(ty, true))
                .transpose()?;
            return self.integer_arithmetic(expr, expected.as_ref());
        }
        self.scalar_input(expr)?;
        let expected = annotation
            .map(|ty| self.source_type(ty, true))
            .transpose()?;
        if expected
            .as_ref()
            .is_some_and(|ty| !matches!(ty, Type::Int { .. } | Type::Bool))
        {
            return Err(Diagnostic::unsupported(
                "non-integer computed scalar bindings",
                expr.span,
            ));
        }
        self.integer_result(expr, expected.as_ref())
    }

    pub(crate) fn integer_result(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<Value> {
        let reach = std::mem::replace(&mut self.reach, crate::flow::TRUE);
        let required = std::mem::replace(&mut self.required, true);
        let result = self.expr(expr, expected);
        self.reach = reach;
        self.required = required;
        let expr = result?;
        let Some(value @ Constant::Int(_)) = self.constant(&expr) else {
            return Err(Diagnostic::unsupported(
                "non-integer computed scalar bindings",
                expr.span,
            ));
        };
        Ok(Value::Static { value, ty: expr.ty })
    }
}
