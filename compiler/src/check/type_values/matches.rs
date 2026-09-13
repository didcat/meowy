use super::{MAX_DEPTH, MAX_WORK, Work};
use crate::ast::{Expr, Span, Stmt, StmtKind};
use crate::check::{Checker, Result, Scope, Value};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

impl Checker {
    pub(crate) fn type_branch_form(
        &mut self,
        stmt: &Stmt,
        depth: usize,
        count: &mut usize,
    ) -> Result<()> {
        *count += 1;
        if depth >= MAX_DEPTH || *count > MAX_WORK || !self.flow.spend(1) {
            return Err(Work::budget(stmt.span));
        }
        match &stmt.kind {
            StmtKind::Bind { mutable: false, .. }
            | StmtKind::TypeAlias {
                exported: false, ..
            }
            | StmtKind::Emit {
                label: None,
                name: None,
                ty: None,
                mutable: false,
                ..
            } => Ok(()),
            StmtKind::Match { arms } => {
                for (condition, body) in arms {
                    if condition.is_none() {
                        return Err(Diagnostic::unsupported(
                            "required matcher fallback arms",
                            stmt.span,
                        ));
                    }
                    self.type_branch_form(body, depth + 1, count)?;
                }
                Ok(())
            }
            _ => Err(Diagnostic::unsupported(
                "required matcher body statement",
                stmt.span,
            )),
        }
    }

    pub(crate) fn type_match(
        &mut self,
        arms: &[(Option<Expr>, Box<Stmt>)],
        span: Span,
        primary: &mut Option<Value>,
    ) -> Result<()> {
        let work = self.type_work.as_mut().unwrap();
        if work.depth >= MAX_DEPTH {
            return Err(Work::budget(span));
        }
        work.depth += 1;
        let result = (|| {
            for (condition, body) in arms {
                let Some(condition) = condition else {
                    return Err(Diagnostic::unsupported(
                        "required matcher fallback arms",
                        span,
                    ));
                };
                let depth = self.type_work.as_ref().unwrap().depth;
                self.type_branch_form(body, depth, &mut 0)?;
                let ty = self.boolean_form(condition, depth, &mut 0)?;
                if ty != Type::Bool {
                    return Err(Self::error(
                        "E215",
                        format!("matcher requires boolean, found {ty:?}"),
                        condition.span,
                    ));
                }
                if self.required_boolean(condition)? {
                    self.scopes.push(Scope::default());
                    let result = self.type_statement(body, primary);
                    self.scopes.pop();
                    result?;
                }
            }
            Ok(())
        })();
        self.type_work.as_mut().unwrap().depth -= 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn conditional_types_select_one_type_without_runtime_storage() {
        for (condition, value) in [("true", "7"), ("false", "\"selected\"")] {
            let source = format!(
                "<T>:{{flag:{condition};|flag|-><int32>;|!flag|-><string>}};value<T>:{value}"
            );
            crate::compile(&source).unwrap();
        }
        let program = crate::compile(
            "<T>:{flag:true;|flag|->{n<uint8>:4;<Local>:<int32[n]>;-><Local>};|!flag|-><string>}",
        )
        .unwrap();
        assert!(program.body.stmts.is_empty());
        assert!(program.locals.is_empty());
        assert!(program.functions.is_empty());
        crate::compile("<T>:{flag:true;|true|flag:false;|flag|-><int32>};v<T>:7").unwrap();
    }

    #[test]
    pub(crate) fn conditional_types_skip_values_but_keep_structural_and_primary_rules() {
        crate::compile("<T>:{|false|-><Missing>;|false|value:missing();|false| |missing|-><Unknown>;|true|-><int32>};v<T>:7").unwrap();
        for (source, code) in [
            ("<T>:{|true|-><int32>;|true|-><string>}", "E205"),
            ("<T>:{|false|-><int32>}", "E211"),
            ("<T>:{|1|-><int32>}", "E215"),
            ("<T>:{|true|<Hidden>:<int32>;-><Hidden>}", "E202"),
            ("<T>:{|true|hidden:4;-><int32[hidden]>}", "E201"),
            ("<T>:{|true|-><int32>;tail:1/0}", "E107"),
            ("<T>:{|false|x:=1;-><int32>}", "B001"),
            ("<T>:{|false|{-><int32>};-><int32>}", "B001"),
            ("<T>:{|false|->named:<int32>;-><int32>}", "B001"),
            (
                "f<int32>:(flag<boolean>){<T>:{|flag|-><int32>};->1}",
                "E211",
            ),
        ] {
            let error = crate::compile(source).unwrap_err().remove(0);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }

    #[test]
    pub(crate) fn conditional_types_restore_scope_and_root_state_after_errors() {
        let mut checker = Checker::new();
        let scopes = checker.scopes.len();
        let parsed = crate::parser::parse("value:{|true|-><Missing>}").unwrap();
        let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        assert_eq!(checker.type_value(value).unwrap_err().code, "E202");
        assert!(checker.type_work.is_none());
        assert_eq!(checker.scopes.len(), scopes);
        let parsed = crate::parser::parse("value:{|true|-><int32>}").unwrap();
        let StmtKind::Bind { value, .. } = &parsed.stmts[0].kind else {
            panic!("binding")
        };
        assert_eq!(
            checker.type_value(value).unwrap(),
            Type::Int {
                bits: 32,
                signed: true
            }
        );
    }
    #[test]
    pub(crate) fn conditional_types_bound_matcher_depth_and_restore_failed_roots() {
        use crate::ast::{Block, ExprKind, TypeExpr, TypeKind};
        for depth in [62, 63] {
            let span = Span::new(0, 1);
            let value = Expr {
                span,
                kind: ExprKind::TypeValue(TypeExpr {
                    span,
                    kind: TypeKind::Name("int32".into()),
                }),
            };
            let mut stmt = Stmt {
                span,
                kind: StmtKind::Emit {
                    label: None,
                    name: None,
                    ty: None,
                    mutable: false,
                    value,
                },
            };
            for _ in 0..depth {
                stmt = Stmt {
                    span,
                    kind: StmtKind::Match {
                        arms: vec![(
                            Some(Expr {
                                span,
                                kind: ExprKind::Name("true".into()),
                            }),
                            Box::new(stmt),
                        )],
                    },
                };
            }
            let expr = Expr {
                span,
                kind: ExprKind::Block(Block {
                    span,
                    label: None,
                    stmts: vec![stmt],
                }),
            };
            let mut checker = Checker::new();
            let scopes = checker.scopes.len();
            let result = checker.type_value(&expr);
            if depth == 62 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert_eq!(result.unwrap_err().code, "B001");
            }
            assert!(checker.type_work.is_none());
            assert_eq!(checker.scopes.len(), scopes);
        }
    }

    #[test]
    pub(crate) fn conditional_types_document_selected_declarations_without_claiming_skipped_checks()
    {
        let source =
            "#| Choice. |#<T>:{|true|->{#| Number. |#<Local>:<int32>;-><Local>};|false|-><string>}";
        let (_, model) = crate::documentation::checked(source, true).unwrap();
        let model = model.unwrap();
        assert_eq!(
            model
                .entries
                .iter()
                .find(|entry| entry.name == "T")
                .unwrap()
                .signature,
            "int32"
        );
        assert!(
            model
                .entries
                .iter()
                .find(|entry| entry.name == "Local")
                .unwrap()
                .checked
        );
        let source = "<T>:{|false|->{#| Skipped. |#<Local>:<int32>;-><Local>};-><int32>}";
        let error = crate::documentation::checked(source, true)
            .unwrap_err()
            .remove(0);
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("unanalyzed declaration"));
    }
}
