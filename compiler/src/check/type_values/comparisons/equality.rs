use crate::ast::{Expr, ExprKind, Span};
use crate::check::{Checker, Constant, Result, Value};
use crate::hir::Type;

impl Checker {
    pub(crate) fn block_equality_form(
        &mut self,
        op: &str,
        left: &Expr,
        right: &Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<Option<Type>> {
        let context = self
            .required_hint(left)
            .or_else(|| self.required_hint(right))
            .map(|ty| Self::primary_type(&ty));
        if context
            .as_ref()
            .is_some_and(|ty| !matches!(ty, Type::Int { .. } | Type::Bool))
        {
            return Err(Self::error(
                "E222",
                "required block equality needs scalar operands",
                left.span,
            ));
        }
        let a = self.equality_operand_form(left, context.as_ref(), depth, count)?;
        let b = self.equality_operand_form(right, a.as_ref().or(context.as_ref()), depth, count)?;
        if let (Some(a), Some(b)) = (&a, &b) {
            Self::equality_kinds(op, a, b, Span::new(left.span.start, right.span.end))?;
        }
        Ok(context)
    }

    pub(crate) fn equality_operand_form(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
        depth: usize,
        count: &mut usize,
    ) -> Result<Option<Type>> {
        match &expr.kind {
            ExprKind::Block(block) => {
                self.form_work(expr, depth, count)?;
                self.required_block_form(block, "comparison", depth, count)?;
                Ok(expected.cloned())
            }
            ExprKind::Group(value) => {
                self.form_work(expr, depth, count)?;
                self.equality_operand_form(value, expected, depth + 1, count)
            }
            _ if self.boolean_scalar(expr) => {
                let ty = Self::primary_type(&self.boolean_form(expr, depth, count)?);
                if let Some(expected) = expected {
                    Self::equality_kinds("==", &ty, expected, expr.span)?;
                }
                Ok(Some(ty))
            }
            _ => self.block_integer_form(expr, expected, depth, count),
        }
    }

    pub(crate) fn equality_kinds(op: &str, left: &Type, right: &Type, span: Span) -> Result<()> {
        if matches!((left, right), (Type::Int { .. }, Type::Int { .. })) {
            return Self::integer_operands(op, left, right, span);
        }
        if left != right {
            return Err(Self::error(
                "E222",
                "required equality operands have different scalar kinds",
                span,
            ));
        }
        Ok(())
    }

    pub(crate) fn equality_operand(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
    ) -> Result<Value> {
        let value = if matches!(expr.kind, ExprKind::Block(_) | ExprKind::Group(_)) {
            self.type_work.as_mut().unwrap().enter(expr.span)?;
            let result = match &expr.kind {
                ExprKind::Block(_) => self.operand_block(expr, expected),
                ExprKind::Group(value) => self.equality_operand(value, expected),
                _ => unreachable!(),
            };
            self.type_work.as_mut().unwrap().depth -= 1;
            result?
        } else if self.boolean_scalar(expr) {
            self.type_boolean(expr, None)?
        } else {
            self.integer_arithmetic(expr, expected.filter(|ty| matches!(ty, Type::Int { .. })))?
        };
        let ty = value.data_type();
        if !matches!(ty, Some(Type::Int { .. } | Type::Bool))
            || expected.is_some_and(|expected| ty.as_ref() != Some(expected))
        {
            return Err(Self::error(
                "E207",
                "required equality operand does not match its scalar context",
                expr.span,
            ));
        }
        Ok(value)
    }

    pub(crate) fn required_block_equality(
        &mut self,
        op: &str,
        left: &Expr,
        right: &Expr,
        context: Option<&Type>,
    ) -> Result<bool> {
        let left = self.equality_operand(left, context)?;
        let ty = left.data_type().unwrap();
        let right = self.equality_operand(right, Some(&ty))?;
        match (left, right) {
            (
                Value::Static {
                    value: Constant::Int(a),
                    ..
                },
                Value::Static {
                    value: Constant::Int(b),
                    ..
                },
            ) => Ok(Self::compare_integers(op, a, b)),
            (
                Value::Static {
                    value: Constant::Bool(a),
                    ..
                },
                Value::Static {
                    value: Constant::Bool(b),
                    ..
                },
            ) => Ok(if op == "==" { a == b } else { a != b }),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn block_equality_selects_boolean_and_integer_kinds_without_runtime_storage() {
        for a in [false, true] {
            for b in [false, true] {
                for (op, expected) in [("==", a == b), ("!=", a != b)] {
                    let source = format!(
                        "<T>:{{flag:({{->{a}}}){op}({{->{b}}});|flag|-><int32[4]>;|!flag|-><int32[2]>}}"
                    );
                    let program = crate::compile(&source).unwrap();
                    assert!(program.locals.is_empty());
                    assert!(program.body.stmts.is_empty());
                    let values = if expected { "1,2,3,4" } else { "1,2" };
                    crate::compile(&format!("{source};v<T>:[{values}]")).unwrap();
                }
            }
        }
        crate::compile("<T>:{flag:true==({->true});other:({->true})!=false;integer:({v<uint8>:4;->v})==4;|flag&&other&&integer|-><int32>};v<T>:7").unwrap();
    }

    #[test]
    pub(crate) fn block_equality_defers_skipped_values_and_preserves_domain_gates() {
        crate::compile("<T>:{flag:false&&(({x<Missing>:unknown();->x})==true);other:true||(({->1/0})==({->false}));-><int32>}").unwrap();
        for (body, code) in [
            ("flag:({->true})==1", "E207"),
            ("flag:({->1})==false", "E207"),
            ("flag:({->true})<({->false})", "E207"),
            ("flag:({->true})>false", "E222"),
            ("flag:({->true;->false})==true", "E205"),
            ("flag:({local:true;->local})==local", "E201"),
            ("flag:({-><boolean>})==({->true})", "E207"),
            ("flag:false&&(({local:=true;->local})==true)", "B001"),
        ] {
            let error = crate::compile(&format!("<T>:{{{body};-><int32>}}"))
                .unwrap_err()
                .remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use crate::ast::StmtKind;
    use crate::check::type_values::{MAX_DEPTH, MAX_WORK, Work};

    #[test]
    pub(crate) fn block_equality_preserves_selected_work_and_restores_failed_state() {
        for (source, expected, cost) in [
            ("({->false})==({->false})", true, 11),
            ("({->false})!=({->true})", true, 11),
            ("({local:true;->local})==({->true})", true, 13),
            ("false&&(({->missing()})==true)", false, 2),
        ] {
            let parsed = crate::parser::parse(&format!("flag:{source}")).unwrap();
            let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
                panic!("binding")
            };
            let mut checker = Checker::new();
            checker.type_work = Some(Work::default());
            let scopes = checker.scopes.len();
            checker.boolean_form(value, 0, &mut 0).unwrap();
            assert_eq!(checker.type_work.as_ref().unwrap().visits, 0);
            assert!(
                matches!(checker.type_boolean(value,None).unwrap(),Value::Static {value:Constant::Bool(found),..} if found==expected)
            );
            assert_eq!(checker.type_work.as_ref().unwrap().visits, cost, "{source}");
            for (visits, depth, accepted) in [
                (MAX_WORK - cost, 0, true),
                (MAX_WORK - cost + 1, 0, false),
                (0, MAX_DEPTH, false),
            ] {
                checker.type_work = Some(Work {
                    visits,
                    depth,
                    nodes: 0,
                });
                let result = checker.type_boolean(value, None);
                if accepted {
                    assert!(result.is_ok(), "{:?}", result.err());
                } else {
                    assert_eq!(result.err().unwrap().code, "B001");
                }
                assert_eq!(checker.scopes.len(), scopes);
                assert_eq!(checker.type_work.as_ref().unwrap().depth, depth);
                assert!(!checker.required);
            }
            assert!(checker.locals.is_empty());
        }
    }

    #[test]
    pub(crate) fn block_equality_preserves_error_order_and_documents_boolean_results() {
        for (expr, failed) in [
            ("({->false})==({->bad})", "raw.n+1"),
            ("({->true})!=({->bad})", "raw.n+1"),
            ("({->false;tail:1/0})==({->bad})", "1/0"),
            ("({->bad})==({->false;tail:1/0})", "raw.n+1"),
        ] {
            let source =
                format!("raw:{{->n<uint8>:255}};bad:raw.n+1==0;<T>:{{flag:{expr};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, "E107");
            assert_eq!(error.span.start, source.find(failed).unwrap());
        }
        let source = "#| Items. |#<T>:{#| Same. |#flag:({#| Local. |#local:true;->local})==true;|flag|-><int32[4]>;|!flag|-><string>}";
        let (_, model) = crate::documentation::checked(source, true).unwrap();
        let model = model.unwrap();
        for (name, signature) in [("flag", "boolean"), ("local", "boolean"), ("T", "int32[4]")] {
            let entry = model
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap();
            assert!(entry.checked);
            assert_eq!(entry.signature, signature);
        }
        let source = "<T>:{flag:false&&(({#| Skipped. |#local:true;->local})==true);-><int32>}";
        assert_eq!(
            crate::documentation::checked(source, true).unwrap_err()[0].code,
            "B001"
        );
    }
}
