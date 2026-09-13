use crate::ast::{self, ExprKind};
use crate::check::{Checker, Result, Value, inputs::Record};
use crate::hir::Type;

impl Checker {
    pub(crate) fn type_record(
        &mut self,
        expr: &ast::Expr,
        annotation: Option<&ast::TypeExpr>,
    ) -> Result<Value> {
        let (ty, input) = self.required_record(expr)?;
        if let Some(annotation) = annotation {
            let expected = self.ty(annotation)?;
            if expected != ty {
                return Err(Self::error(
                    "E207",
                    format!("expected {expected:?}, found {ty:?}"),
                    expr.span,
                ));
            }
        }
        self.type_work
            .as_mut()
            .unwrap()
            .materialize(&ty, expr.span)?;
        Ok(Value::Record {
            ty,
            input: Box::new(input),
        })
    }

    pub(crate) fn required_record(&mut self, expr: &ast::Expr) -> Result<(Type, Record)> {
        self.type_work.as_mut().unwrap().enter(expr.span)?;
        let result = (|| {
            let (ty, input) = match &expr.kind {
                ExprKind::Name(name) => match self.required_value(name, expr.span)? {
                    Value::Local {
                        id,
                        ty,
                        mutable: false,
                        ..
                    } => (ty, self.record_inputs.get(&id).cloned()),
                    Value::Record { ty, input } => (ty, Some(*input)),
                    _ => {
                        return Err(Self::error(
                            "E211",
                            "required record source is not an immutable record",
                            expr.span,
                        ));
                    }
                },
                ExprKind::Field { .. } => {
                    let (source, ty, path) = self.required_path(expr)?;
                    (ty, source.record(self, &path))
                }
                ExprKind::Group(value) => return self.required_record(value),
                _ => return Err(self.type_unavailable(expr)?),
            };
            let mut input = input.filter(|_| self.record_shape(&ty)).ok_or_else(|| {
                Self::error(
                    "E211",
                    "record initializer is unavailable during required type evaluation",
                    expr.span,
                )
            })?;
            self.type_work
                .as_mut()
                .unwrap()
                .input(&input.input, expr.span)?;
            input.input.work = 0;
            Ok((ty, input))
        })();
        self.type_work.as_mut().unwrap().depth -= 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn required_records_keep_leaf_values_and_allocate_no_runtime_locals() {
        let source = "source:{->enabled:false;->part:{->width<uint8>:4}};<T>:{copy:source;part:copy.part;alias:part;flag:copy.enabled;-><int32[alias.width]>}";
        let program = crate::compile(source).unwrap();
        let baseline =
            crate::compile("source:{->enabled:false;->part:{->width<uint8>:4}}").unwrap();
        assert_eq!(program.locals, baseline.locals);
        assert_eq!(program.body.stmts.len(), baseline.body.stmts.len());
        let mut checker = crate::check::inputs::tests::check(
            "->source:{->enabled:false;->part:{->width<uint8>:4}}",
        );
        let id = checker.module.inputs["source"].id;
        let ty = checker.locals[id].clone();
        checker
            .declare(
                "source",
                Value::Local {
                    id,
                    ty,
                    mutable: false,
                    owner: 0,
                    constant: None,
                },
                ast::Span::new(0, 6),
            )
            .unwrap();
        checker.type_work = Some(super::super::Work::default());
        let expr = ast::Expr {
            span: ast::Span::new(0, 6),
            kind: ExprKind::Name("source".into()),
        };
        let Value::Record { input, .. } = checker.type_record(&expr, None).unwrap() else {
            panic!("record")
        };
        assert_eq!(input.input.work, 0);
        assert_eq!(input.boolean(&[0]).unwrap().value, Some(false));
        assert_eq!(input.field(&[1, 0]).unwrap().value, Some(4));
    }

    #[test]
    pub(crate) fn required_records_check_whole_ancestor_evidence_and_scope() {
        for (source, code) in [
            (
                "source:{->good:{->n:4};->bad:=1};<T>:{copy:source.good;-><int32>}",
                "E211",
            ),
            (
                "d:@\"debug\";source:{->n:4;d.print(9)};<T>:{copy:source;-><int32>}",
                "E211",
            ),
            ("source:{->n:4};<T>:{copy:source;->copy}", "E211"),
            ("source:{->n:4};<T>:{copy:source;->copy<>};v:copy", "E201"),
            ("source:{->n:4};<T>:{copy<{}>:source;-><int32>}", "E207"),
            (
                "source:{->n:4};<T>:{copy:source;copy:source;-><int32>}",
                "E203",
            ),
            (
                "source:{->n:4};<T>:{copy:source;flag:copy==copy;-><int32>}",
                "B001",
            ),
        ] {
            let error = crate::compile(source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
        let source = "row:{->n<uint8>:255};source:{->good:{->n:4};->bad:row.n+1};<T>:{copy:source.good;-><int32>}";
        let error = crate::compile(source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107");
        assert_eq!(error.span.start, source.find("row.n+1").unwrap());
    }
}
