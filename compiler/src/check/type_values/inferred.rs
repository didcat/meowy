use crate::ast::{Expr, ExprKind, Span};
use crate::check::{Checker, Result, Value, type_values::Output};
use crate::hir::Type;

impl Checker {
    pub(crate) fn inferred_block(&mut self, expr: &Expr) -> Result<Value> {
        self.type_work.as_mut().unwrap().enter(expr.span)?;
        let result = (|| {
            let block = match &expr.kind {
                ExprKind::Group(value) => return self.inferred_block(value),
                ExprKind::Block(block) => block,
                _ => unreachable!(),
            };
            let output = self.scoped_output(
                block,
                Output {
                    infer: true,
                    ..Output::default()
                },
            )?;
            if output.record() {
                self.finish_required_record(output, block.span)
            } else {
                output.value.ok_or_else(|| {
                    Self::error(
                        "E211",
                        "computed block does not emit a compile-time value",
                        block.span,
                    )
                })
            }
        })();
        self.type_work.as_mut().unwrap().depth -= 1;
        let value = result?;
        if let Value::Type(ty) = &value {
            self.type_work
                .as_mut()
                .unwrap()
                .materialize(ty, expr.span)?;
        }
        Ok(value)
    }

    pub(crate) fn inferred_primary(
        &mut self,
        expr: &Expr,
        span: Span,
        output: &mut Output,
    ) -> Result<()> {
        let mut form = expr;
        while let ExprKind::Group(value) = &form.kind {
            form = value;
        }
        let value = if matches!(form.kind, ExprKind::Block(_)) {
            self.inferred_block(expr)?
        } else if matches!(form.kind, ExprKind::Name(_) | ExprKind::Field { .. })
            && matches!(self.required_hint(expr), Some(Type::Record { .. }))
        {
            self.type_record(expr, None)?
        } else {
            Value::Type(self.type_value(expr)?)
        };
        if matches!(value, Value::Record { .. }) {
            return self.forward_required_record(value, span, output);
        }
        if output.record() {
            return Err(Self::error(
                "E211",
                "a compile-time type cannot be a record primary",
                span,
            ));
        }
        if output.value.replace(value).is_some() {
            return Err(Self::error(
                "E205",
                "computed type primary may be emitted twice",
                span,
            ));
        }
        Ok(())
    }
}
