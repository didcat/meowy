use crate::ast::{Expr, Span, TypeExpr};
use crate::check::{
    Checker, Result, Value,
    type_values::{Output, Work},
};
use crate::diagnostic::Diagnostic;
use crate::hir::{Field, Type};

impl Checker {
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
        if matches!(output.value, Some(Value::Static { .. })) {
            return Err(Diagnostic::unsupported(
                "required records with scalar primaries",
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
    #[test]
    pub(crate) fn inferred_composition_maps_nested_sources_without_forwarded_bindings() {
        let source = "<T>:{base:{->z<uint8>:4;->part:{->width:z}};r:{->base;->a:false};copy:{->({->r})};|!copy.a|-><int32[copy.part.width]>;|copy.a|-><string>}";
        let program = crate::compile(source).unwrap();
        assert!(program.locals.is_empty());
        assert!(program.body.stmts.is_empty());
        crate::compile(&format!("{source};v<T>:[1,2,3,4]")).unwrap();
        for (body, code) in [
            ("->base;->z:4", "E205"),
            ("->z:4;->base", "E205"),
            ("->base;->{->a:true}", "E205"),
            ("->base;->a:z>0", "E201"),
            ("->base;-><int32>", "E211"),
            ("-><int32>;->base", "E211"),
            ("->{->a:4;unused:1/0}", "E107"),
            ("->missing", "E201"),
        ] {
            let source = format!("<T>:{{base:{{->z<uint8>:4}};r:{{{body}}};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
    }
    #[test]
    pub(crate) fn inferred_records_bound_work_and_restore_scope_after_failures() {
        use super::*;
        use crate::ast::StmtKind;
        use crate::check::type_values::{MAX_NODES, MAX_WORK};
        let parsed = crate::parser::parse("r:{local<uint8>:4;->z:local;->a:false}").unwrap();
        let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        let mut checker = Checker::new();
        checker.type_work = Some(Work::default());
        let scopes = checker.scopes.len();
        let Value::Record { input, .. } = checker.inferred_block(value).unwrap() else {
            panic!("record")
        };
        assert_eq!(input.boolean(&[0]).unwrap().value, Some(false));
        assert_eq!(input.field(&[1]).unwrap().value, Some(4));
        assert_eq!(input.input.work, 0);
        let work = checker.type_work.as_ref().unwrap();
        let cost = work.visits;
        let nodes = work.nodes;
        for (visits, nodes, accepted) in [
            (MAX_WORK - cost, MAX_NODES - nodes, true),
            (MAX_WORK - cost + 1, 0, false),
            (0, MAX_NODES - nodes + 1, false),
        ] {
            checker.type_work = Some(Work {
                visits,
                nodes,
                depth: 0,
            });
            let result = checker.inferred_block(value);
            if accepted {
                assert!(result.is_ok(), "{:?}", result.err());
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.scopes.len(), scopes);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            for name in ["local", "a", "z"] {
                assert_eq!(
                    checker.required_value(name, value.span).err().unwrap().code,
                    "E201"
                );
            }
        }
        assert!(checker.locals.is_empty());
    }

    #[test]
    pub(crate) fn inferred_records_bound_fields_and_nested_shapes() {
        use super::*;
        use crate::ast::{Block, ExprKind, Stmt, StmtKind};
        for count in [256, 257] {
            let fields = (0..count)
                .rev()
                .map(|id| format!("->n{id}:1;"))
                .collect::<String>();
            let source = format!("<T>:{{r:{{{fields}}};-><int32[r.n0]>}}");
            let result = crate::compile(&source);
            if count == 256 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert_eq!(result.unwrap_err()[0].code, "B001");
            }
        }
        for depth in [32, 33] {
            let span = Span::new(0, 1);
            let mut expr = Expr {
                span,
                kind: ExprKind::Int("4".into()),
            };
            for _ in 0..depth {
                expr = Expr {
                    span,
                    kind: ExprKind::Block(Block {
                        span,
                        label: None,
                        stmts: vec![Stmt {
                            span,
                            kind: StmtKind::Emit {
                                label: None,
                                name: Some("n".into()),
                                ty: None,
                                mutable: false,
                                value: expr,
                            },
                        }],
                    }),
                };
            }
            let mut checker = Checker::new();
            checker.type_work = Some(Work::default());
            let scopes = checker.scopes.len();
            let result = checker.inferred_block(&expr);
            if depth == 32 {
                assert!(result.is_ok(), "{:?}", result.err());
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.scopes.len(), scopes);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            assert!(checker.locals.is_empty());
        }
    }

    #[test]
    pub(crate) fn inferred_records_document_selected_fields_and_preserve_skipped_gates() {
        let source = "#| Items. |#<T>:{kind:{|false|->missing;-><uint8>};r:{#| Width. |#->z<(kind)>:4;|false|->absent:missing();->a:false};-><int32[r.z]>}";
        let (_, model) = crate::documentation::checked(source, true).unwrap();
        let model = model.unwrap();
        for (name, signature) in [("z", "uint8"), ("T", "int32[4]")] {
            let entry = model
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap();
            assert!(entry.checked);
            assert_eq!(entry.signature, signature);
        }
        for (body, code) in [
            ("r:{|false|->absent:missing();->n:4};v:r.absent", "E201"),
            ("r:{|false|->{#| Skipped. |#->n:4};->n:4}", "B001"),
            ("r:{|false|->n:=4;->m:4}", "B001"),
        ] {
            let source = format!("<T>:{{{body};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
    }
}
