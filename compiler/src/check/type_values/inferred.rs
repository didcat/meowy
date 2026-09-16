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
            self.type_work.as_mut().unwrap().type_result(ty, expr)?;
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
    #[test]
    pub(crate) fn inferred_scalars_share_work_nodes_and_restore_failed_scopes() {
        use super::*;
        use crate::ast::StmtKind;
        use crate::check::Constant;
        use crate::check::type_values::{MAX_NODES, MAX_WORK, Work};
        use crate::hir::Type;
        let parsed = crate::parser::parse("n:{<Byte>:<uint8>;base<Byte>:4;->base}").unwrap();
        let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        let mut checker = Checker::new();
        checker.type_work = Some(Work::default());
        let scopes = checker.scopes.len();
        assert!(matches!(
            checker.inferred_block(value).unwrap(),
            Value::Static {
                value: Constant::Int(4),
                ty: Type::Int {
                    bits: 8,
                    signed: false
                },
            }
        ));
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
                ..Work::default()
            });
            let result = checker.inferred_block(value);
            if accepted {
                assert!(result.is_ok(), "{:?}", result.err());
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.scopes.len(), scopes);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            assert_eq!(
                checker
                    .required_value("base", value.span)
                    .err()
                    .unwrap()
                    .code,
                "E201"
            );
        }
        assert!(checker.locals.is_empty());
    }

    #[test]
    pub(crate) fn inferred_scalars_bound_nested_primaries_and_keep_type_roots_separate() {
        use super::*;
        use crate::ast::{Block, Stmt, StmtKind};
        use crate::check::type_values::Work;
        for leaf in ["4", "true"] {
            for depth in [63, 64] {
                let span = Span::new(0, 1);
                let mut expr = Expr {
                    span,
                    kind: if leaf == "4" {
                        ExprKind::Int(leaf.into())
                    } else {
                        ExprKind::Name(leaf.into())
                    },
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
                                    name: None,
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
                if depth == 63 {
                    assert!(result.is_ok(), "{:?}", result.err());
                } else {
                    assert_eq!(result.err().unwrap().code, "B001");
                }
                assert_eq!(checker.scopes.len(), scopes);
                assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
                assert!(checker.locals.is_empty());
            }
        }
        for (source, code) in [("<T>:{->4}", "B001"), ("<T>:{->true}", "E211")] {
            assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
        }
    }

    #[test]
    pub(crate) fn inferred_scalars_document_selected_kinds_and_checked_widths() {
        for (flag, signature) in [("true", "uint8"), ("false", "int32")] {
            let source = format!(
                "#| Items. |#<T>:{{flag:{{->{flag}}};#| Count. |#n:{{|flag|->{{base<uint8>:4;->base}};|!flag|->2}};-><int32[n]>}}"
            );
            let (_, model) = crate::documentation::checked(&source, true).unwrap();
            let model = model.unwrap();
            let entry = model
                .entries
                .iter()
                .find(|entry| entry.name == "n")
                .unwrap();
            assert!(entry.checked);
            assert_eq!(entry.signature, signature);
        }
        let source = "<T>:{n:{|false|->{#| Skipped. |#base:4;->base};->4};-><int32[n]>}";
        assert_eq!(
            crate::documentation::checked(source, true).unwrap_err()[0].code,
            "B001"
        );
    }
}
