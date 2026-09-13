use crate::ast::{self, ExprKind};
use crate::check::{Checker, Constant, Result, Value};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn required_integer(&mut self, expr: &ast::Expr, ty: &Type) -> Result<i128> {
        self.scalar_input(expr)?;
        let Value::Static {
            value: Constant::Int(value),
            ..
        } = self.integer_result(expr, Some(ty))?
        else {
            unreachable!()
        };
        Ok(value)
    }

    pub(crate) fn integer_comparison_form(
        &mut self,
        op: &str,
        left: &ast::Expr,
        right: &ast::Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<Option<Type>> {
        let a = self.required_hint(left);
        let b = self.required_hint(right);
        if matches!(op, "==" | "!=")
            && matches!(
                (&a, &b),
                (Some(Type::Record { .. }), Some(Type::Record { .. }))
            )
        {
            return Ok(None);
        }
        let context = a.or(b).map(|ty| Self::primary_type(&ty));
        if context
            .as_ref()
            .is_some_and(|ty| !matches!(ty, Type::Int { .. }))
        {
            return Ok(None);
        }
        let a = self.integer_form(left, context.as_ref(), depth, count)?;
        let b = self.integer_form(right, Some(&a), depth, count)?;
        Self::integer_operands(op, &a, &b, ast::Span::new(left.span.start, right.span.end))?;
        Ok(Some(a))
    }

    pub(crate) fn integer_operands(
        op: &str,
        left: &Type,
        right: &Type,
        span: ast::Span,
    ) -> Result<()> {
        if left != right {
            return Err(Self::error(
                "E213",
                format!(
                    "operator `{op}` requires compatible operands, found {left:?} and {right:?}"
                ),
                span,
            ));
        }
        Ok(())
    }

    pub(crate) fn integer_form(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
        depth: usize,
        count: &mut usize,
    ) -> Result<Type> {
        self.form_work(expr, depth, count)?;
        let ty = match &expr.kind {
            ExprKind::Int(text) => self.integer(text, false, expected, expr.span)?.ty,
            ExprKind::Name(name) => match self.required_value(name, expr.span)? {
                Value::Local { ty, .. }
                | Value::Static { ty, .. }
                | Value::FileModule { ty, .. } => Self::primary_type(&ty),
                Value::Constant(value) => Self::constant_expr(value, expr.span).ty,
                _ => {
                    return Err(Self::error(
                        "E222",
                        "required integer operand is not a scalar value",
                        expr.span,
                    ));
                }
            },
            ExprKind::Field { .. } => Self::primary_type(&self.required_path(expr)?.1),
            ExprKind::Group(value) => return self.integer_form(value, expected, depth + 1, count),
            ExprKind::Unary { op, value } if matches!(op.as_str(), "-" | "~") => {
                if op == "-"
                    && let ExprKind::Int(text) = &value.kind
                {
                    self.form_work(value, depth + 1, count)?;
                    return self
                        .integer(text, true, expected, expr.span)
                        .map(|value| value.ty);
                }
                let context = self.required_hint(value).map(|ty| Self::primary_type(&ty));
                let ty =
                    self.integer_form(value, context.as_ref().or(expected), depth + 1, count)?;
                if op == "-" && matches!(ty, Type::Int { signed: false, .. }) {
                    return Err(Self::error(
                        "E222",
                        "unsigned integer negation is not defined",
                        expr.span,
                    ));
                }
                ty
            }
            ExprKind::Binary { op, left, right }
                if matches!(op.as_str(), "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^") =>
            {
                let context = self
                    .required_hint(left)
                    .or_else(|| self.required_hint(right))
                    .map(|ty| Self::primary_type(&ty));
                let a = self.integer_form(left, context.as_ref().or(expected), depth + 1, count)?;
                let b = self.integer_form(right, Some(&a), depth + 1, count)?;
                Self::integer_operands(op, &a, &b, expr.span)?;
                a
            }
            _ => return Err(self.type_unavailable(expr)?),
        };
        if ty == Type::Bool {
            return Err(Self::error(
                "E222",
                "required integer operand is boolean",
                expr.span,
            ));
        }
        if !matches!(ty, Type::Int { .. }) {
            return Err(Diagnostic::unsupported(
                "required comparison operands outside integers",
                expr.span,
            ));
        }
        Ok(ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn expression(source: &str) -> ast::Expr {
        let block = crate::parser::parse(&format!("value:{source}")).unwrap();
        let ast::StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
            panic!("binding")
        };
        value.clone()
    }

    pub(crate) fn checker() -> Checker {
        let mut checker = Checker::new();
        for (name, bits, signed) in [
            ("small", 8, false),
            ("wide", 16, false),
            ("signed", 8, true),
        ] {
            let ty = Type::Int { bits, signed };
            let id = checker.local(ty.clone());
            checker
                .declare(
                    name,
                    Value::Local {
                        id,
                        ty,
                        mutable: false,
                        owner: 0,
                        constant: None,
                    },
                    ast::Span::new(0, 1),
                )
                .unwrap();
        }
        checker
    }

    #[test]
    pub(crate) fn comparison_forms_preserve_widths_and_skip_initializer_evidence() {
        for (left, right, bits, signed) in [
            ("1", "small", 8, false),
            ("small+1", "2", 8, false),
            ("-128", "signed", 8, true),
            ("1+2", "wide", 16, false),
            ("~small", "small", 8, false),
        ] {
            let mut checker = checker();
            assert_eq!(
                checker
                    .integer_comparison_form("<", &expression(left), &expression(right), 0, &mut 0)
                    .unwrap(),
                Some(Type::Int { bits, signed })
            );
            assert!(checker.inputs.is_empty());
        }
        for (left, right, code) in [
            ("small", "wide", "E213"),
            ("small", "256", "E216"),
            ("256", "small", "E216"),
            ("-1", "small", "E222"),
            ("small+wide", "2", "E213"),
            ("missing", "small", "E201"),
        ] {
            let mut checker = checker();
            let error = checker
                .integer_comparison_form("<", &expression(left), &expression(right), 0, &mut 0)
                .unwrap_err();
            assert_eq!(error.code, code, "{left} < {right}: {error:?}");
        }
    }
    #[test]
    pub(crate) fn required_integer_comparisons_keep_values_and_erase_scratch() {
        use crate::check::type_values::Work;
        for (a, b) in [(-3, 4), (4, 4), (7, -2)] {
            for (op, result) in [
                ("==", a == b),
                ("!=", a != b),
                ("<", a < b),
                (">", a > b),
                ("<=", a <= b),
                (">=", a >= b),
            ] {
                let mut checker = Checker::new();
                checker.type_work = Some(Work::default());
                assert!(
                    matches!(checker.type_scalar(&expression(&format!("{a}{op}{b}")), None).unwrap(),
                        Value::Static { value: Constant::Bool(value), .. } if value == result
                    )
                );
            }
        }
        let program = crate::compile("<T>:{a<uint8>:3;flag:a+1==4;->flag<>}").unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
    }
}
