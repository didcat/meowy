use crate::ast::{self, ExprKind};
use crate::check::{Checker, Result, Value};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn required_block_form(
        &mut self,
        block: &ast::Block,
        kind: &str,
        depth: usize,
        count: &mut usize,
    ) -> Result<()> {
        if block.label.is_some() {
            return Err(Diagnostic::unsupported(
                format!("labeled required {kind} blocks"),
                block.span,
            ));
        }
        for stmt in &block.stmts {
            self.type_branch_form(stmt, false, depth + 1, count)?;
        }
        Ok(())
    }

    pub(crate) fn operand_block(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<Value> {
        match expected {
            Some(ty @ (Type::Int { .. } | Type::Bool)) => self.scalar_block(expr, ty),
            None => self.inferred_block(expr),
            _ => Err(Diagnostic::unsupported(
                "required operand block result kind",
                expr.span,
            )),
        }
    }

    pub(crate) fn scalar_block(&mut self, expr: &ast::Expr, ty: &Type) -> Result<Value> {
        self.type_work.as_mut().unwrap().enter(expr.span)?;
        let result = match &expr.kind {
            ExprKind::Group(value) => self.scalar_block(value, ty),
            ExprKind::Block(block) => self.required_block(block, Some(ty)),
            _ => unreachable!(),
        };
        self.type_work.as_mut().unwrap().depth -= 1;
        result
    }

    pub(crate) fn scalar_emission(&mut self, expr: &ast::Expr, ty: &Type) -> Result<Value> {
        let mut form = expr;
        while let ExprKind::Group(value) = &form.kind {
            form = value;
        }
        if matches!(form.kind, ExprKind::Block(_)) {
            return self.scalar_block(expr, ty);
        }
        if self.type_operand_form(expr, self.type_work.as_ref().unwrap().depth, &mut 0)? {
            self.type_value(expr)?;
            return Err(Self::error(
                "E207",
                "required scalar block cannot emit a type value",
                expr.span,
            ));
        }
        if self.boolean_scalar(expr) {
            let value = self.type_boolean(expr, None)?;
            if *ty != Type::Bool {
                return Err(Self::error(
                    "E207",
                    format!("expected {ty:?}, found {:?}", Type::Bool),
                    expr.span,
                ));
            }
            return Ok(value);
        }
        if self.integer_blocks(expr)? {
            return self.integer_arithmetic(expr, Some(ty));
        }
        self.scalar_input(expr)?;
        self.integer_result(expr, Some(ty))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Constant;

    pub(crate) fn expression(source: &str) -> ast::Expr {
        let parsed = crate::parser::parse(&format!("value:{source}")).unwrap();
        let ast::StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        value.clone()
    }

    #[test]
    pub(crate) fn operand_blocks_keep_contextual_and_inferred_scalar_kinds() {
        let mut checker = Checker::new();
        checker.type_work = Some(super::super::Work::default());
        let byte = Type::Int {
            bits: 8,
            signed: false,
        };
        let scopes = checker.scopes.len();
        for (source, expected, ty) in [
            ("{->4}", Some(&byte), byte.clone()),
            ("{->true}", Some(&Type::Bool), Type::Bool),
            (
                "{->4}",
                None,
                Type::Int {
                    bits: 32,
                    signed: true,
                },
            ),
        ] {
            let value = checker
                .operand_block(&expression(source), expected)
                .unwrap();
            assert_eq!(value.data_type(), Some(ty));
            assert_eq!(checker.scopes.len(), scopes);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
        }
        assert!(checker.locals.is_empty());
    }

    #[test]
    pub(crate) fn operand_block_forms_do_not_resolve_or_evaluate_initializers() {
        let expr = expression("{local<Missing>:unknown();->local}");
        let ExprKind::Block(block) = &expr.kind else {
            panic!("block")
        };
        let mut checker = Checker::new();
        let scopes = checker.scopes.len();
        let mut count = 0;
        checker
            .required_block_form(block, "comparison", 0, &mut count)
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(checker.scopes.len(), scopes);
        assert!(checker.type_work.is_none());
        assert!(checker.locals.is_empty());
        assert_eq!(
            checker
                .required_value("local", expr.span)
                .err()
                .unwrap()
                .code,
            "E201"
        );
    }

    #[test]
    pub(crate) fn required_scalar_blocks_keep_values_widths_and_no_runtime_storage() {
        let mut checker = Checker::new();
        checker.type_work = Some(super::super::Work::default());
        let expr = expression("{->{->false};unused:7}");
        assert!(matches!(
            checker.scalar_block(&expr, &Type::Bool).unwrap(),
            Value::Static {
                value: Constant::Bool(false),
                ty: Type::Bool
            }
        ));
        let expr = expression("{n<uint8>:{->2};->n+2}");
        assert!(matches!(
            checker
                .scalar_block(
                    &expr,
                    &Type::Int {
                        bits: 8,
                        signed: false
                    }
                )
                .unwrap(),
            Value::Static {
                value: Constant::Int(4),
                ty: Type::Int {
                    bits: 8,
                    signed: false
                }
            }
        ));
        let program = crate::compile(
            "<T>:{flag<boolean>:{->true};n<uint8>:({|flag|->{->4};|!flag|->2});-><int32[n]>}",
        )
        .unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
        assert!(program.functions.is_empty());
    }

    #[test]
    pub(crate) fn required_scalar_blocks_preserve_primary_kind_scope_and_tail_errors() {
        for (body, code) in [
            ("n<uint8>:{->true}", "E207"),
            ("n<boolean>:{->1}", "E207"),
            ("n<uint8>:{->256}", "E216"),
            ("n<uint8>:{v<uint8>:255;->v+1}", "E107"),
            ("n<uint8>:{->1;->2}", "E205"),
            ("n<boolean>:{|false|->true}", "E204"),
            ("n<uint8>:{->1;unused:1/0}", "E107"),
            ("n<uint8>:{private<uint8>:4;->private};copy:private", "E201"),
            ("n<float32>:{->1.0}", "B001"),
            ("n<uint8>:{v:=1;->v}", "B001"),
            ("n<uint8>:{|true|{->4}}", "B001"),
        ] {
            let source = format!("<T>:{{{body};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }
    #[test]
    pub(crate) fn required_scalar_blocks_restore_depth_scope_and_results_after_failures() {
        use super::super::Work;
        use crate::ast::{Block, Expr, Span, Stmt, StmtKind};
        for depth in [63, 64] {
            let span = Span::new(0, 1);
            let mut expr = Expr {
                span,
                kind: ExprKind::Int("1".into()),
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
            let result = checker.scalar_block(
                &expr,
                &Type::Int {
                    bits: 8,
                    signed: false,
                },
            );
            if depth == 63 {
                assert!(matches!(
                    result.unwrap(),
                    Value::Static {
                        value: Constant::Int(1),
                        ..
                    }
                ));
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.scopes.len(), scopes);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            assert!(checker.locals.is_empty());
            assert!(matches!(
                checker
                    .scalar_block(&expression("{->false}"), &Type::Bool)
                    .unwrap(),
                Value::Static {
                    value: Constant::Bool(false),
                    ..
                }
            ));
        }
    }

    #[test]
    pub(crate) fn required_scalar_blocks_document_checked_widths_and_constructed_types() {
        let source = "#| Items. |#<T>:{#| Count. |#n<uint8>:{#| Base. |#base<uint8>:2;->base*2};-><int32[n]>}";
        let (_, model) = crate::documentation::checked(source, true).unwrap();
        let model = model.unwrap();
        for (name, signature) in [("T", "int32[4]"), ("n", "uint8"), ("base", "uint8")] {
            let entry = model
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap();
            assert!(entry.checked);
            assert_eq!(entry.signature, signature);
        }
    }
}
