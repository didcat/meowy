use crate::ast::{Expr, ExprKind};
use crate::check::{Checker, Constant, Result, Value};
use crate::diagnostic::Diagnostic;
use crate::hir::{self, Type};

impl Checker {
    pub(crate) fn integer_blocks(&mut self, expr: &Expr) -> Result<bool> {
        let mut pending = vec![(expr, 0)];
        let mut count = 0;
        while let Some((expr, depth)) = pending.pop() {
            self.form_work(expr, depth, &mut count)?;
            match &expr.kind {
                ExprKind::Block(_) => return Ok(true),
                ExprKind::Group(value) => pending.push((value, depth + 1)),
                ExprKind::Unary { op, value } if matches!(op.as_str(), "-" | "~") => {
                    pending.push((value, depth + 1));
                }
                ExprKind::Binary { op, left, right }
                    if matches!(op.as_str(), "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^") =>
                {
                    pending.push((right, depth + 1));
                    pending.push((left, depth + 1));
                }
                _ => {}
            }
        }
        Ok(false)
    }

    pub(crate) fn integer_arithmetic(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
    ) -> Result<Value> {
        if expected.is_some_and(|ty| !matches!(ty, Type::Int { .. } | Type::Bool)) {
            return Err(Diagnostic::unsupported(
                "non-integer computed scalar bindings",
                expr.span,
            ));
        }
        let reach = std::mem::replace(&mut self.reach, crate::flow::TRUE);
        let required = std::mem::replace(&mut self.required, true);
        let result = self.integer_operand(expr, expected);
        self.reach = reach;
        self.required = required;
        let mut value = result?;
        if let Some(expected) = expected {
            value = Self::expected_value(value, expected, expr.span)?;
        }
        let Some(value) = self.constant(&value).map(|constant| Value::Static {
            value: constant,
            ty: value.ty,
        }) else {
            return Err(Self::error(
                "E211",
                "required integer operand has no checked value",
                expr.span,
            ));
        };
        Ok(value)
    }

    pub(crate) fn integer_operand(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
    ) -> Result<hir::Expr> {
        self.type_work.as_mut().unwrap().enter(expr.span)?;
        let result = (|| match &expr.kind {
            ExprKind::Block(_) => {
                let value = if let Some(ty @ Type::Int { .. }) = expected {
                    self.scalar_block(expr, ty)?
                } else {
                    self.inferred_block(expr)?
                };
                match value {
                    Value::Static {
                        value: Constant::Int(value),
                        ty,
                    } => Ok(hir::Expr {
                        kind: hir::ExprKind::Int(value),
                        ty,
                        span: expr.span,
                    }),
                    _ => Err(Self::error(
                        "E207",
                        "required arithmetic block must produce an integer",
                        expr.span,
                    )),
                }
            }
            ExprKind::Group(value) => {
                let value = self.integer_operand(value, expected)?;
                match expected {
                    Some(ty) => Self::expected_value(value, ty, expr.span),
                    None => Ok(value),
                }
            }
            ExprKind::Unary { op, value }
                if matches!(op.as_str(), "-" | "~")
                    && !(op == "-" && matches!(value.kind, ExprKind::Int(_))) =>
            {
                let context = self.unary_context(op, value, expected)?;
                let value = self.integer_operand(value, context.as_ref())?;
                self.unary_value(op, value, expr.span)
            }
            ExprKind::Binary { op, left, right }
                if matches!(op.as_str(), "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^") =>
            {
                let context = self
                    .required_hint(left)
                    .or_else(|| self.required_hint(right))
                    .map(|ty| Self::primary_type(&ty))
                    .or_else(|| expected.map(Self::primary_type));
                let left = self.integer_operand(left, context.as_ref())?;
                let right = self.integer_operand(right, Some(&left.ty))?;
                self.binary_values(op, left, right, expr.span)
            }
            _ => {
                self.scalar_input(expr)?;
                self.expression(expr, expected)
            }
        })();
        self.type_work.as_mut().unwrap().depth -= 1;
        let value = result?;
        if !matches!(value.ty, Type::Int { .. }) {
            return Err(Self::error(
                "E207",
                "required arithmetic operand must be an integer",
                expr.span,
            ));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn required_integer_blocks_preserve_context_and_erase_runtime_storage() {
        let source = "<T>:{n<uint8>:({base<uint8>:2;->base})+{->2};m:~({v<uint8>:255;->v});s:-({->2})+6;copy:{->n+({->m})};-><int32[copy+4]>}";
        let program = crate::compile(source).unwrap();
        assert!(program.locals.is_empty());
        assert!(program.body.stmts.is_empty());
        crate::compile(&format!("{source};v<T>:[1,2,3,4,5,6,7,8]")).unwrap();
        crate::compile("<T>:{-><int32[({->2})+2]>};v<T>:[1,2,3,4]").unwrap();
    }

    #[test]
    pub(crate) fn required_integer_blocks_keep_errors_scope_and_unsupported_kinds() {
        for (body, code) in [
            ("n<uint8>:({->256})+1", "E216"),
            ("n<int32><null>:({->2})+2", "B001"),
            ("n<uint8>:({->255})+1", "E107"),
            ("n:({->1})/0", "E107"),
            ("n:-({v<int8>:-128;->v})", "E107"),
            ("n:-({v<uint8>:1;->v})", "E222"),
            ("n:({->1;->2})+1", "E205"),
            ("n:({secret:4;->secret})+secret", "E201"),
            ("n:({->false})+1", "E207"),
            ("n:({-><int32>})+1", "E207"),
            ("n:({->x:4})+1", "E207"),
            ("n:({->1;tail:1/0})+1", "E107"),
            ("n:({->1})==1", "B001"),
            ("n:false&&({->true})", "B001"),
        ] {
            let source = format!("<T>:{{{body};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
    }
}
