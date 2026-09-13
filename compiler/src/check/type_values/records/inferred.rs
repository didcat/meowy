use crate::ast::{Expr, ExprKind, Span, TypeExpr};
use crate::check::{
    Checker, Result, Value,
    type_values::{Output, Work},
};
use crate::diagnostic::Diagnostic;
use crate::hir::{Field, Type};

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

    pub(crate) fn inferred_field(
        &mut self,
        name: &str,
        annotation: Option<&TypeExpr>,
        expr: &Expr,
        span: Span,
        output: &mut Output,
    ) -> Result<()> {
        let value = if let Some(annotation) = annotation {
            let ty = self.ty(annotation)?;
            if !self.record_shape(&ty) {
                return Err(Diagnostic::unsupported(
                    "inferred required field shape",
                    span,
                ));
            }
            self.required_field_value(expr, &ty)?
        } else {
            self.type_binding(expr, None)?
        };
        let ty = value.data_type().ok_or_else(|| {
            Self::error("E211", "required record field is not a data value", span)
        })?;
        let index = self.inferred_slot(name, ty, span, output)?;
        self.insert_required_field(name, index, value, true, span, output)
    }

    pub(crate) fn inferred_slot(
        &mut self,
        name: &str,
        ty: Type,
        span: Span,
        output: &mut Output,
    ) -> Result<usize> {
        if matches!(output.value, Some(Value::Type(_))) {
            return Err(Self::error(
                "E211",
                "a compile-time type cannot be a record primary",
                span,
            ));
        }
        let shape = output.ty.get_or_insert_with(|| Type::Record {
            primary: Box::new(Type::Null),
            fields: Vec::new(),
        });
        let Type::Record { fields, .. } = shape else {
            unreachable!()
        };
        if !self.flow.spend(fields.len() + name.len() + 1) {
            return Err(Work::budget(span));
        }
        let index = match fields.binary_search_by(|field| field.name.as_str().cmp(name)) {
            Ok(index) => return Ok(index),
            Err(index) => index,
        };
        fields.insert(
            index,
            Field {
                name: name.into(),
                ty,
                mutable: false,
            },
        );
        if !self.record_shape(shape) {
            return Err(Diagnostic::unsupported(
                "inferred required record shape",
                span,
            ));
        }
        let tail = output.fields.split_off(&index);
        output
            .fields
            .extend(tail.into_iter().map(|(index, value)| (index + 1, value)));
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn inferred_records_keep_sorted_paths_widths_and_type_bindings() {
        let source = "<T>:{kind:{-><uint8>};r:({->z<(kind)>:4;->part:{->width:z};->enabled:part.width>0});copy:r;|copy.enabled|-><int32[copy.part.width]>;|!copy.enabled|-><string>}";
        let program = crate::compile(source).unwrap();
        assert!(program.locals.is_empty());
        assert!(program.body.stmts.is_empty());
        crate::compile(&format!("{source};v<T>:[1,2,3,4]")).unwrap();
    }

    #[test]
    pub(crate) fn inferred_records_preserve_field_and_result_gates() {
        for (body, code) in [
            ("->x:4;->x:true", "E205"),
            ("->x<uint8>:256", "E216"),
            ("->x<uint8>:true", "E207"),
            ("->x<float32>:1.0", "B001"),
            ("->x:=4", "B001"),
            ("->x:<int32>", "E211"),
            ("-><int32>;->x:4", "E211"),
            ("->x:4;unused:1/0", "E107"),
            ("|true|->x:4;->y:x", "E201"),
            ("->4", "B001"),
            ("", "E211"),
        ] {
            let source = format!("<T>:{{r:{{{body}}};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
        assert_eq!(crate::compile("<T>:{->x:4}").unwrap_err()[0].code, "B001");
        assert_eq!(
            crate::compile("<T>:{r:{->x:4};n:x;-><int32>}").unwrap_err()[0].code,
            "E201"
        );
    }
}
