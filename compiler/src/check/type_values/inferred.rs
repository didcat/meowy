use crate::ast::{Expr, ExprKind, Span};
use crate::check::{Checker, Result, Value, type_values::Output};
use crate::diagnostic::Diagnostic;

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
        let value = self.type_binding(expr, None)?;
        if matches!(value, Value::Record { .. }) {
            return self.forward_required_record(value, span, output);
        }
        if output.record() {
            if matches!(value, Value::Static { .. }) {
                return Err(Diagnostic::unsupported(
                    "required records with scalar primaries",
                    span,
                ));
            }
            return Err(Self::error(
                "E211",
                "a compile-time type cannot be a record primary",
                span,
            ));
        }
        if output.value.replace(value).is_some() {
            return Err(Self::error(
                "E205",
                "inferred required primary may be emitted twice",
                span,
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn inferred_scalars_keep_widths_nested_blocks_and_type_results() {
        let source = "<T>:{kind:{-><uint8>};n:({base<(kind)>:2;->{->base*2}});flag:{->n==4};copy:{->flag};|copy|-><int32[n]>;|!copy|-><string>}";
        let program = crate::compile(source).unwrap();
        assert!(program.locals.is_empty());
        assert!(program.body.stmts.is_empty());
        assert!(program.functions.is_empty());
        crate::compile(&format!("{source};v<T>:[1,2,3,4]")).unwrap();
        crate::compile("<T>:{r:{->n:{->4};->flag:{->true}};-><int32[r.n]>}").unwrap();
    }

    #[test]
    pub(crate) fn inferred_scalars_preserve_primary_scope_and_record_boundaries() {
        for (body, code) in [
            ("n:{->1;->false}", "E205"),
            ("n:{-><int32>;->4}", "E205"),
            ("n:{->1/0}", "E107"),
            ("n:{->4;tail:1/0}", "E107"),
            ("n:{base<uint8>:255;->base+1}", "E107"),
            ("n:{->2147483648}", "E216"),
            ("n:{private:4;->private};copy:private", "E201"),
            ("n:{|false|->4}", "E211"),
            ("n:{->x:4;->1}", "B001"),
            ("n:{->1;->x:4}", "B001"),
            ("n:{->true;->{->x:4}}", "E205"),
            ("n:{->{->x:4};->true}", "B001"),
            ("n:{->1.0}", "B001"),
            ("n:{->null}", "B001"),
            ("n:{->\"text\"}", "B001"),
            ("n:{x:=4;->x}", "B001"),
            ("n:{->4};wrong<uint8>:n", "E207"),
        ] {
            let source = format!("<T>:{{{body};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
        assert_eq!(
            crate::compile("<T>:{n:{->4};->n}").unwrap_err()[0].code,
            "E211"
        );
    }
}
