use crate::ast::{Expr, ExprKind, Span};
use crate::check::{Checker, Result, Value};

impl Checker {
    pub(crate) fn type_comparison_form(
        &mut self,
        op: &str,
        left: &Expr,
        right: &Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<bool> {
        let a = self.type_operand_form(left, depth, count)?;
        let b = self.type_operand_form(right, depth, count)?;
        if !a && !b {
            return Ok(false);
        }
        if matches!(op, "==" | "!=")
            && (a && Self::equality_block(right) || b && Self::equality_block(left))
        {
            return Ok(false);
        }
        if !a || !b || !matches!(op, "==" | "!=") {
            return Err(Self::error(
                "E222",
                "type values support only equality with another type value",
                Span::new(left.span.start, right.span.end),
            ));
        }
        Ok(true)
    }

    pub(crate) fn equality_block(mut expr: &Expr) -> bool {
        while let ExprKind::Group(value) = &expr.kind {
            expr = value;
        }
        matches!(expr.kind, ExprKind::Block(_))
    }

    pub(crate) fn type_operand_form(
        &mut self,
        expr: &Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<bool> {
        self.form_work(expr, depth, count)?;
        match &expr.kind {
            ExprKind::Binary { op, left, right } if op == "!" => {
                self.subtraction_operand_form(left, depth + 1, count)?;
                self.subtraction_operand_form(right, depth + 1, count)?;
                Ok(true)
            }
            ExprKind::TypeValue(_) | ExprKind::TypeQuery(_) => Ok(true),
            ExprKind::Group(value) => self.type_operand_form(value, depth + 1, count),
            ExprKind::Name(_) | ExprKind::Field { .. } => {
                let mut base = expr;
                let mut depth = depth;
                while let ExprKind::Field { value, .. } | ExprKind::Group(value) = &base.kind {
                    base = value;
                    depth += 1;
                    self.form_work(base, depth, count)?;
                }
                if !matches!(base.kind, ExprKind::Name(_) | ExprKind::Import(_)) {
                    return Ok(false);
                }
                let saved = std::mem::replace(&mut self.required, true);
                let symbol = self.symbol(expr);
                self.required = saved;
                Ok(matches!(
                    symbol?,
                    Some(Value::Type(_) | Value::Foundation(crate::foundation::Item::Type(_)))
                ))
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn type_equality_uses_normalized_identity_without_runtime_storage() {
        for (left, right, same) in [
            ("<int32>", "<int32>", true),
            ("<int32>", "<uint32>", false),
            ("<int32><null><never>", "<null><int32><int32>", true),
            ("<{a<int32>;b<boolean>}>", "<{b<boolean>;a<int32>}>", true),
            ("<{a<int32>}>", "<{a<int32>:=}>", false),
            ("<int32[2]>", "<int32[1+1]>", true),
            ("<int32[2]>", "<int32[3]>", false),
            ("<&int32>", "<&!int32>", false),
        ] {
            for (op, expected) in [("==", same), ("!=", !same)] {
                let source =
                    format!("<T>:{{flag:{left} {op} {right};|flag|-><int32>;|!flag|-><boolean>}}");
                let program = crate::compile(&source).unwrap();
                assert!(program.locals.is_empty());
                assert!(program.body.stmts.is_empty());
                let value = if expected { "7" } else { "true" };
                crate::compile(&format!("{source};v<T>:{value}")).unwrap();
            }
        }
        crate::compile(
            "kind<Type>:<int32>;<T>:{copy:kind;flag:(copy)==7<>;|flag|-><int32>};v<T>:7",
        )
        .unwrap();
    }

    #[test]
    pub(crate) fn type_equality_skips_construction_but_checks_operand_kinds() {
        crate::compile("<T>:{flag:false&&(<int32[1/0]> == <Missing>);other:true||(<(missing())> != <int32>);-><int32>}").unwrap();
        for (expr, code) in [
            ("<int32> == true", "E222"),
            ("1!=<int32>", "E222"),
            ("false&&(<int32> == false)", "E222"),
            ("<int32> < <int32>", "E222"),
            ("<int32> == missing", "E201"),
            ("<int32> == <Missing>", "E202"),
            ("<int32> == <int32[1/0]>", "E107"),
        ] {
            let source = format!("<T>:{{flag:{expr};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }
}

#[cfg(test)]
mod work {
    use super::*;
    use crate::ast::StmtKind;
    use crate::check::Constant;
    use crate::check::type_values::{MAX_DEPTH, MAX_NODES, MAX_WORK, Work};

    #[test]
    pub(crate) fn type_equality_charges_selected_operands_and_restores_failed_state() {
        for (source, visits, nodes) in [
            ("<int32> == <int32>", 3, 2),
            ("false&&(<int32[1/0]> == <Missing>)", 2, 0),
            ("true||(<int32[1/0]> != <Missing>)", 2, 0),
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
            assert_eq!(checker.type_work.as_ref().unwrap().nodes, 0);
            checker.type_boolean(value, None).unwrap();
            assert_eq!(checker.type_work.as_ref().unwrap().visits, visits);
            assert_eq!(checker.type_work.as_ref().unwrap().nodes, nodes);
            for (work, accepted) in [
                (
                    Work {
                        visits: MAX_WORK - visits,
                        nodes: MAX_NODES - nodes,
                        depth: 0,
                        ..Work::default()
                    },
                    true,
                ),
                (
                    Work {
                        visits: MAX_WORK - visits + 1,
                        ..Work::default()
                    },
                    false,
                ),
                (
                    Work {
                        depth: MAX_DEPTH,
                        ..Work::default()
                    },
                    false,
                ),
            ] {
                let depth = work.depth;
                checker.type_work = Some(work);
                let result = checker.type_boolean(value, None);
                if accepted {
                    assert!(result.is_ok(), "{source}: {:?}", result.err());
                } else {
                    assert_eq!(result.err().unwrap().code, "B001");
                }
                assert_eq!(checker.scopes.len(), scopes);
                assert_eq!(checker.type_work.as_ref().unwrap().depth, depth);
                assert!(!checker.required);
            }
            if nodes > 0 {
                checker.type_work = Some(Work {
                    nodes: MAX_NODES - nodes + 1,
                    ..Work::default()
                });
                assert_eq!(
                    checker.type_boolean(value, None).err().unwrap().code,
                    "B001"
                );
                assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            }
            checker.type_work = Some(Work::default());
            assert!(matches!(
                checker.type_boolean(value, None).unwrap(),
                Value::Static {
                    value: Constant::Bool(_),
                    ..
                }
            ));
            assert!(checker.locals.is_empty());
        }
    }

    #[test]
    pub(crate) fn type_equality_charges_the_same_cached_input_on_both_sides() {
        use crate::check::inputs::Input;
        use crate::hir::Type;
        for (right, accepted) in [("4", true), ("n", false)] {
            let parsed =
                crate::parser::parse(&format!("flag:<int32[n]> == <int32[{right}]>")).unwrap();
            let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
                panic!("binding")
            };
            let mut checker = Checker::new();
            checker
                .declare(
                    "n",
                    Value::Local {
                        id: 0,
                        ty: Type::Int {
                            bits: 32,
                            signed: true,
                        },
                        mutable: false,
                        owner: 0,
                        constant: None,
                    },
                    value.span,
                )
                .unwrap();
            checker.inputs.insert(
                0,
                Input {
                    derived: false,
                    value: Some(4),
                    error: None,
                    work: MAX_WORK / 2,
                },
            );
            checker.type_work = Some(Work::default());
            let result = checker.type_boolean(value, None);
            if accepted {
                assert!(result.is_ok(), "{:?}", result.err());
            } else {
                assert_eq!(result.err().unwrap().code, "B001");
            }
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
        }
    }

    #[test]
    pub(crate) fn type_equality_retains_left_to_right_constructor_errors() {
        for (expr, failed) in [
            ("<int32[1/0]> == <int32[2/0]>", "1/0"),
            ("<int32[2/0]> != <int32[1/0]>", "2/0"),
            ("<int32> == <({-><int32>;tail:2/0})>", "2/0"),
            ("<({-><int32>;tail:1/0})> == <int32[2/0]>", "1/0"),
        ] {
            let source = format!("<T>:{{flag:{expr};-><int32>}}");
            let error = crate::compile(&source).unwrap_err().remove(0);
            assert_eq!(error.code, "E107", "{expr}: {error:?}");
            assert_eq!(error.span.start, source.find(failed).unwrap());
        }
    }
}
