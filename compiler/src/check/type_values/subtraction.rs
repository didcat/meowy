use crate::ast::{Expr, ExprKind};
use crate::check::{Checker, Result, Value};
use crate::hir::Type;

impl Checker {
    pub(crate) fn subtraction_operand_form(
        &mut self,
        expr: &Expr,
        depth: usize,
        count: &mut usize,
    ) -> Result<()> {
        match &expr.kind {
            ExprKind::Group(value) => {
                self.form_work(expr, depth, count)?;
                self.subtraction_operand_form(value, depth + 1, count)
            }
            ExprKind::Block(block) => {
                self.form_work(expr, depth, count)?;
                self.required_block_form(block, "type subtraction", depth, count)
            }
            _ if self.type_operand_form(expr, depth, count)? => Ok(()),
            _ => Err(Self::error(
                "E222",
                "type subtraction requires compile-time type operands",
                expr.span,
            )),
        }
    }

    pub(crate) fn subtraction_operand(&mut self, expr: &Expr) -> Result<Type> {
        if !Self::equality_block(expr) {
            return self.type_value(expr);
        }
        match self.inferred_block(expr)? {
            Value::Type(ty) => Ok(ty),
            _ => Err(Self::error(
                "E207",
                "type subtraction block must produce a type value",
                expr.span,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn type_subtraction_removes_normalized_alternatives_without_storage() {
        for (expr, expected) in [
            ("<int32><null>!<null>", "<int32>"),
            ("<int32>!<uint32>", "<int32>"),
            ("<int32>!<int32>", "<never>"),
            ("<never>!<int32>", "<never>"),
            ("<int32>!<never>", "<int32>"),
            ("<int32><null><boolean>!<null>!<boolean>", "<int32>"),
            ("<int32><null><boolean>!<(<null><boolean>)>", "<int32>"),
            ("<{a<int32>;b<boolean>}>!<{b<boolean>;a<int32>}>", "<never>"),
            ("<{a<int32>}>!<{a<int32>:=}>", "<{a<int32>}>"),
            ("<&int32>!<&!int32>", "<&int32>"),
            ("<int32[2]>!<int32[1+1]>", "<never>"),
            ("({local:<int32><null>;->local})!<null>", "<int32>"),
        ] {
            let source = format!(
                "kind<Type>:{expr};<T>:{{copy:kind!<never>;same:copy=={expected};|same|-><int32>}}"
            );
            let program = crate::compile(&source).unwrap();
            assert!(program.body.stmts.is_empty());
            assert!(program.locals.is_empty());
            assert!(program.functions.is_empty());
            crate::compile(&format!("{source};v<T>:7")).unwrap();
        }
        crate::compile("v<int32><null>!<null>:7").unwrap();
    }

    #[test]
    pub(crate) fn type_subtraction_keeps_kinds_queries_and_runtime_values_checked() {
        crate::compile("f<int32>:(v<uint8>){<T>:v<>!<null>;<R>:{same:<T> == <uint8>;|same|-><int32>};n<R>:7;->n}").unwrap();
        for (source, code) in [
            ("kind<Type>:1!<int32>", "E222"),
            ("kind<Type>:({->true})!<null>", "E207"),
            ("kind<Type>:<int32>!<Missing>", "E202"),
            ("core:@\"core\";kind<Type>:<int32>!<core.error>", "B001"),
            ("kind<Type>:<int32>!<1>", "E004"),
            ("v<int32><null>:null;present<(v<>!<null>)>:v", "E207"),
            ("f<int32>:(v<int32><null>){n<(v<>!<null>)>:v;->n}", "E207"),
        ] {
            let error = crate::compile(source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use crate::ast::{Span, StmtKind};
    use crate::check::type_values::{MAX_DEPTH, MAX_NODES, MAX_WORK, Work};

    #[test]
    pub(crate) fn type_subtraction_skips_constructors_but_checks_known_operand_forms() {
        for expr in [
            "false&&(<int32[1/0]>!<Missing> == <never>)",
            "true||(({local<Missing>:unknown();->local})!<null> == <int32>)",
            "false&&(({->true})!<null> == <never>)",
            "false&&(({kind:<int32>;->kind})!<(kind)> == <never>)",
        ] {
            crate::compile(&format!("<T>:{{flag:{expr};-><int32>}}")).unwrap();
        }
        for (expr, code) in [
            ("false&&(1!<null> == <never>)", "E222"),
            ("true||(missing!<null> == <never>)", "E201"),
            ("false&&(({local:=1;-><int32>})!<null> == <never>)", "B001"),
            ("({kind:<int32>;->kind})!<(kind)> == <never>", "E201"),
        ] {
            let source = format!("<T>:{{flag:{expr};-><int32>}}");
            assert_eq!(
                crate::compile(&source).unwrap_err()[0].code,
                code,
                "{source}"
            );
        }
    }

    #[test]
    pub(crate) fn type_subtraction_charges_both_types_and_restores_budget_state() {
        for (removed, expected) in [
            (
                "null",
                Type::Int {
                    bits: 32,
                    signed: true,
                },
            ),
            ("int32", Type::Never),
        ] {
            let parsed = crate::parser::parse(&format!("value:kind!<{removed}>")).unwrap();
            let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
                panic!("binding")
            };
            let mut checker = Checker::new();
            checker
                .declare(
                    "kind",
                    Value::Type(Type::Int {
                        bits: 32,
                        signed: true,
                    }),
                    Span::new(0, 4),
                )
                .unwrap();
            checker.type_work = Some(Work::default());
            checker.type_operand_form(value, 0, &mut 0).unwrap();
            assert_eq!(checker.type_work.as_ref().unwrap().visits, 0);
            assert_eq!(checker.type_work.as_ref().unwrap().nodes, 0);
            assert_eq!(checker.type_value(value).unwrap(), expected);
            assert_eq!(checker.type_work.as_ref().unwrap().visits, 3);
            assert_eq!(checker.type_work.as_ref().unwrap().nodes, 3);
            let scopes = checker.scopes.len();
            for (work, accepted) in [
                (
                    Work {
                        visits: MAX_WORK - 3,
                        nodes: MAX_NODES - 3,
                        depth: 0,
                    },
                    true,
                ),
                (
                    Work {
                        visits: MAX_WORK - 2,
                        ..Work::default()
                    },
                    false,
                ),
                (
                    Work {
                        nodes: MAX_NODES - 2,
                        ..Work::default()
                    },
                    false,
                ),
                (
                    Work {
                        depth: MAX_DEPTH - 1,
                        ..Work::default()
                    },
                    false,
                ),
            ] {
                let depth = work.depth;
                checker.type_work = Some(work);
                let result = checker.type_value(value);
                if accepted {
                    assert_eq!(result.unwrap(), expected);
                } else {
                    assert_eq!(result.unwrap_err().code, "B001");
                }
                assert_eq!(checker.type_work.as_ref().unwrap().depth, depth);
                assert_eq!(checker.scopes.len(), scopes);
                assert!(!checker.required);
            }
            checker.type_work = Some(Work::default());
            assert_eq!(checker.type_value(value).unwrap(), expected);
            assert!(checker.locals.is_empty());
        }
    }
}
