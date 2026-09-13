use crate::ast::{Expr, ExprKind, Span};
use crate::check::{Checker, Result};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn block_comparison_form(
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
        let a = self.block_integer_form(left, context.as_ref(), depth, count)?;
        let b = self.block_integer_form(right, a.as_ref().or(context.as_ref()), depth, count)?;
        if let (Some(a), Some(b)) = (&a, &b) {
            Self::integer_operands(op, a, b, Span::new(left.span.start, right.span.end))?;
        }
        Ok(context)
    }

    pub(crate) fn block_integer_form(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
        depth: usize,
        count: &mut usize,
    ) -> Result<Option<Type>> {
        self.form_work(expr, depth, count)?;
        if expected.is_some_and(|ty| !matches!(ty, Type::Int { .. })) {
            return Err(Self::error(
                "E222",
                "required block comparison needs integer operands",
                expr.span,
            ));
        }
        match &expr.kind {
            ExprKind::Block(block) => {
                if block.label.is_some() {
                    return Err(Diagnostic::unsupported(
                        "labeled required comparison blocks",
                        block.span,
                    ));
                }
                for stmt in &block.stmts {
                    self.type_branch_form(stmt, false, depth + 1, count)?;
                }
                Ok(expected.cloned())
            }
            ExprKind::Group(value) => self.block_integer_form(value, expected, depth + 1, count),
            ExprKind::Int(text) => expected
                .map(|ty| {
                    self.integer(text, false, Some(ty), expr.span)
                        .map(|value| value.ty)
                })
                .transpose(),
            ExprKind::Unary { op, value } if matches!(op.as_str(), "-" | "~") => {
                if op == "-"
                    && let ExprKind::Int(text) = &value.kind
                {
                    self.form_work(value, depth + 1, count)?;
                    return expected
                        .map(|ty| {
                            self.integer(text, true, Some(ty), expr.span)
                                .map(|value| value.ty)
                        })
                        .transpose();
                }
                let context = self.required_hint(value).map(|ty| Self::primary_type(&ty));
                let ty = self.block_integer_form(
                    value,
                    context.as_ref().or(expected),
                    depth + 1,
                    count,
                )?;
                if op == "-" && matches!(ty, Some(Type::Int { signed: false, .. })) {
                    return Err(Self::error(
                        "E222",
                        "unsigned integer negation is not defined",
                        expr.span,
                    ));
                }
                Ok(ty)
            }
            ExprKind::Binary { op, left, right }
                if matches!(op.as_str(), "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^") =>
            {
                let context = self
                    .required_hint(left)
                    .or_else(|| self.required_hint(right))
                    .map(|ty| Self::primary_type(&ty));
                let a =
                    self.block_integer_form(left, context.as_ref().or(expected), depth + 1, count)?;
                let b = self.block_integer_form(
                    right,
                    a.as_ref().or(context.as_ref()).or(expected),
                    depth + 1,
                    count,
                )?;
                if let (Some(a), Some(b)) = (&a, &b) {
                    Self::integer_operands(op, a, b, expr.span)?;
                }
                Ok(a.or(b))
            }
            _ => self.integer_form(expr, expected, depth, count).map(Some),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn block_comparisons_select_all_integer_relations_without_runtime_storage() {
        for (op, expected) in [
            ("==", false),
            ("!=", true),
            ("<", true),
            (">", false),
            ("<=", true),
            (">=", false),
        ] {
            let source = format!(
                "<T>:{{flag:({{base<uint8>:2;->base}}){op}({{->3}});|flag|-><int32[4]>;|!flag|-><int32[2]>}}"
            );
            let program = crate::compile(&source).unwrap();
            assert!(program.locals.is_empty());
            assert!(program.body.stmts.is_empty());
            let values = if expected { "1,2,3,4" } else { "1,2" };
            crate::compile(&format!("{source};v<T>:[{values}]")).unwrap();
        }
        crate::compile(
            "<T>:{small<uint8>:4;flag:({->2})+2==small;|flag|-><int32>;|!flag|-><string>};v<T>:7",
        )
        .unwrap();
    }

    #[test]
    pub(crate) fn block_comparisons_defer_local_values_but_keep_outer_and_statement_checks() {
        for source in [
            "<T>:{flag:false&&(({->missing()})==({->1/0}));-><int32>}",
            "<T>:{flag:true||(({v<uint16>:4;->v})==65536);-><int32>}",
            "<T>:{flag:false&&(({->true})==({->false}));-><int32>}",
        ] {
            crate::compile(source).unwrap();
        }
        for (body, code) in [
            ("flag:({->missing})==1", "E201"),
            ("flag:({v<uint16>:4;->v})==65536", "E216"),
            ("small<uint8>:4;flag:({v<uint16>:4;->v})==small", "E207"),
            ("flag:({->true})==1", "E207"),
            ("flag:({->1;->2})==1", "E205"),
            ("flag:({x:4;->x})==x", "E201"),
            ("flag:false&&(({x:=4;->x})==1)", "B001"),
            ("flag:false&&(({->field:4})==1)", "B001"),
            ("flag:false&&(({->4})==unknown)", "E201"),
            ("flag:({->4})==false", "E222"),
        ] {
            let source = format!("<T>:{{{body};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{body}: {error:?}");
        }
    }
    #[test]
    pub(crate) fn block_comparisons_charge_only_selected_operands_once() {
        use super::*;
        use crate::ast::StmtKind;
        use crate::check::{
            Constant, Value,
            type_values::{MAX_WORK, Work},
        };
        let parsed = crate::parser::parse("flag:({local:1;->2})==({->2})").unwrap();
        let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        let mut checker = Checker::new();
        checker.type_work = Some(Work::default());
        let scopes = checker.scopes.len();
        assert_eq!(checker.boolean_form(value, 0, &mut 0).unwrap(), Type::Bool);
        assert_eq!(checker.type_work.as_ref().unwrap().visits, 0);
        assert_eq!(checker.scopes.len(), scopes);
        assert!(matches!(
            checker.type_boolean(value, None).unwrap(),
            Value::Static {
                value: Constant::Bool(true),
                ..
            }
        ));
        assert_eq!(checker.type_work.as_ref().unwrap().visits, 13);
        for (visits, accepted) in [(MAX_WORK - 13, true), (MAX_WORK - 12, false)] {
            checker.type_work = Some(Work {
                visits,
                ..Work::default()
            });
            let result = checker.type_boolean(value, None);
            if accepted {
                assert!(result.is_ok(), "{:?}", result.err());
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.scopes.len(), scopes);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            assert!(!checker.required);
        }
        let parsed =
            crate::parser::parse("flag:false&&(({v<Missing>:unknown();->v})==65536)").unwrap();
        let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        checker.type_work = Some(Work::default());
        assert!(matches!(
            checker.type_boolean(value, None).unwrap(),
            Value::Static {
                value: Constant::Bool(false),
                ..
            }
        ));
        assert_eq!(checker.type_work.as_ref().unwrap().visits, 2);
        assert_eq!(checker.type_work.as_ref().unwrap().nodes, 0);
        assert_eq!(
            checker
                .required_value("local", value.span)
                .err()
                .unwrap()
                .code,
            "E201"
        );
        assert!(checker.locals.is_empty());
    }

    #[test]
    pub(crate) fn block_comparison_forms_bound_skipped_statements_and_depth() {
        use super::*;
        use crate::ast::StmtKind;
        use crate::check::type_values::Work;
        let parsed = crate::parser::parse("value:{local:4;->local}").unwrap();
        let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        for count in [4094, 4096] {
            let mut expr = value.clone();
            let ExprKind::Block(block) = &mut expr.kind else {
                panic!("block")
            };
            block.stmts = vec![block.stmts[0].clone(); count];
            let mut checker = Checker::new();
            checker.type_work = Some(Work::default());
            let result = checker.block_integer_form(&expr, None, 0, &mut 0);
            if count == 4094 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert_eq!(result.unwrap_err().code, "B001");
            }
            assert_eq!(checker.type_work.as_ref().unwrap().visits, 0);
        }
        for groups in [62, 63] {
            let mut expr = value.clone();
            for _ in 0..groups {
                expr = Expr {
                    span: expr.span,
                    kind: ExprKind::Group(Box::new(expr)),
                };
            }
            let mut checker = Checker::new();
            checker.type_work = Some(Work::default());
            let result = checker.block_integer_form(&expr, None, 0, &mut 0);
            if groups == 62 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert_eq!(result.unwrap_err().code, "B001");
            }
            assert_eq!(checker.type_work.as_ref().unwrap().visits, 0);
        }
    }

    #[test]
    pub(crate) fn block_comparisons_preserve_error_order_and_document_selected_values() {
        for (expr, failed) in [
            ("({->1/0})==bad", "1/0"),
            ("bad==({->1/0})", "raw.n+1"),
            ("({->4;tail:1/0})!=({->bad})", "1/0"),
        ] {
            let source =
                format!("raw:{{->n<uint8>:255}};bad:raw.n+1;<T>:{{flag:{expr};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, "E107");
            assert_eq!(error.span.start, source.find(failed).unwrap());
        }
        let source = "#| Items. |#<T>:{#| Ready. |#flag:({#| Base. |#base<uint8>:4;->base})==4;|flag|-><int32[4]>;|!flag|-><string>}";
        let (_, model) = crate::documentation::checked(source, true).unwrap();
        let model = model.unwrap();
        for (name, signature) in [("base", "uint8"), ("flag", "boolean"), ("T", "int32[4]")] {
            let entry = model
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap();
            assert!(entry.checked);
            assert_eq!(entry.signature, signature);
        }
        let source = "<T>:{flag:false&&(({#| Skipped. |#local:4;->local})==4);-><int32>}";
        assert_eq!(
            crate::documentation::checked(source, true).unwrap_err()[0].code,
            "B001"
        );
    }
}
