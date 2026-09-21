use super::Pending;
use crate::ast::{Stmt, StmtKind};
use crate::check::{Checker, Result, Spec, Value};
use crate::foundation::Descriptor;

impl Checker {
    pub(crate) fn pending_statement(&mut self, stmt: &Stmt) -> Result<bool> {
        let value = match &stmt.kind {
            StmtKind::Bind { value, .. } | StmtKind::Expr(value) => value,
            _ => return Ok(false),
        };
        let Some(form) = self.pending_form(value)? else {
            return Ok(false);
        };
        self.construction_root(stmt.span, |checker| {
            checker.type_work.as_mut().unwrap().logical.charge(1, 0)?;
            if matches!(form, Pending::Copy(_)) {
                checker.type_work.as_mut().unwrap().logical.charge(1, 0)?;
            }
            let id = checker.prepare_query(form)?;
            if let StmtKind::Bind {
                name, ty, mutable, ..
            } = &stmt.kind
            {
                if *mutable {
                    return Err(Self::error(
                        "E223",
                        "proof descriptors cannot enter mutable storage",
                        stmt.span,
                    ));
                }
                if let Some(ty) = ty {
                    match checker.source_spec(ty, true)? {
                        Spec::Descriptor(Descriptor::Result) => {}
                        Spec::Descriptor(_) => {
                            return Err(Self::error(
                                "E207",
                                "pending query has declared type proof.Result",
                                ty.span,
                            ));
                        }
                        _ => {
                            return Err(Self::error(
                                "E223",
                                "proof descriptors cannot enter runtime storage",
                                ty.span,
                            ));
                        }
                    }
                }
                checker.declare(name, Value::Pending(id), stmt.span)?;
            }
            Ok(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::check::queries::accounting::checker;
    use crate::check::required::MAX_STEPS;

    #[test]
    pub(crate) fn pending_statements_keep_annotations_in_query_roots() {
        for (source, steps, types) in [
            ("r:p.can_copy<uint32>()", 3, 1),
            ("r<p.Result>:(p.can_copy<uint32>())", 4, 2),
            ("(p.can_copy<uint32>())", 3, 1),
        ] {
            let mut checker = checker();
            let block = crate::parser::parse(source).unwrap();
            checker.stmt(&block.stmts[0]).unwrap();
            let budget = checker.query_budgets[0].as_ref().unwrap();
            assert_eq!(budget.root, block.stmts[0].span);
            assert_eq!((budget.steps, budget.types), (steps, types));
            assert!(checker.type_work.is_none());
            assert!(checker.locals.is_empty());
        }
    }

    #[test]
    pub(crate) fn pending_copy_reads_share_limits_without_replaying_queries() {
        for tail in ["copy:((r))", "((r))"] {
            for left in [1, 2] {
                let mut checker = checker();
                let block =
                    crate::parser::parse(&format!("r:p.can_copy<uint32>();{tail}")).unwrap();
                checker.stmt(&block.stmts[0]).unwrap();
                let root = block.stmts[1].span;
                let result = checker.construction_root(root, |checker| {
                    checker
                        .type_work
                        .as_mut()
                        .unwrap()
                        .logical
                        .charge(MAX_STEPS - left, 0)?;
                    checker.stmt(&block.stmts[1])?;
                    assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, MAX_STEPS);
                    Ok(())
                });
                assert_eq!(result.is_ok(), left == 2);
                if let Err(error) = result {
                    assert_eq!(error.code, "E220");
                    assert_eq!(error.span, root);
                }
                assert_eq!(checker.queries.len(), 1);
                assert_eq!(checker.query_budgets.len(), 1);
                assert_eq!(checker.query_budgets[0].as_ref().unwrap().steps, 3);
                assert!(checker.type_work.is_none());
            }
        }
    }

    #[test]
    pub(crate) fn pending_statement_classification_leaves_ordinary_work_outside_roots() {
        let mut checker = checker();
        for source in ["n:7", "p.revision", "r:p.can_copy<Missing,uint32>()"] {
            let block = crate::parser::parse(source).unwrap();
            let result = checker.pending_statement(&block.stmts[0]);
            if source.contains("Missing") {
                assert_eq!(result.unwrap_err().code, "E212");
            } else {
                assert!(!result.unwrap());
            }
            assert!(checker.type_work.is_none());
            assert!(checker.queries.is_empty());
            assert!(checker.query_budgets.is_empty());
        }
    }

    #[test]
    pub(crate) fn required_branches_preserve_descriptor_admission_gates() {
        for (condition, accepted) in [("false", true), ("true", false)] {
            let source =
                format!(r#"p:@"proof";<T>:{{|{condition}|r:p.can_copy<uint32>();-><int32>}}"#);
            let result = crate::compile(&source);
            assert_eq!(result.is_ok(), accepted, "{result:?}");
            if let Err(errors) = result {
                assert_eq!(errors[0].code, "B001");
                assert!(!errors[0].message.contains("proof.can_copy evaluation"));
            }
        }
    }
}
