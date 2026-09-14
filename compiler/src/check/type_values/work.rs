use super::{MAX_DEPTH, MAX_NODES, MAX_WORK};
use crate::ast::Span;
use crate::check::{Checker, Result, inputs::Input, required::Budget};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

#[derive(Default)]
pub(crate) struct Work {
    pub(crate) visits: usize,
    pub(crate) depth: usize,
    pub(crate) nodes: usize,
    pub(crate) logical: Budget,
}

impl Work {
    pub(crate) fn budget(span: Span) -> Diagnostic {
        Diagnostic::unsupported("computed type bootstrap budget exhausted", span)
    }

    pub(crate) fn spend(&mut self, span: Span) -> Result<()> {
        self.visits += 1;
        if self.visits > MAX_WORK {
            return Err(Self::budget(span));
        }
        Ok(())
    }

    pub(crate) fn enter(&mut self, span: Span) -> Result<()> {
        self.spend(span)?;
        if self.depth == MAX_DEPTH {
            return Err(Self::budget(span));
        }
        self.depth += 1;
        Ok(())
    }

    pub(crate) fn input<T>(&mut self, input: &Input<T>, span: Span) -> Result<()> {
        self.visits = self.visits.saturating_add(input.work);
        if self.visits > MAX_WORK {
            return Err(Self::budget(span));
        }
        match &input.error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    pub(crate) fn node(&mut self, span: Span) -> Result<()> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(Self::budget(span));
        }
        self.logical.charge(1, 1)
    }

    pub(crate) fn materialize(&mut self, ty: &Type, span: Span) -> Result<()> {
        let mut pending = vec![ty];
        while let Some(ty) = pending.pop() {
            self.node(span)?;
            if pending.len() > MAX_NODES {
                return Err(Self::budget(span));
            }
            match ty {
                Type::Reference(ty) | Type::Exclusive(ty) | Type::List { element: ty, .. } => {
                    pending.push(ty);
                }
                Type::Record { primary, fields } => {
                    pending.push(primary);
                    pending.extend(fields.iter().map(|field| &field.ty));
                }
                Type::Union(members) => pending.extend(members),
                _ => {}
            }
        }
        Ok(())
    }
}

impl Checker {
    pub(crate) fn required_root<T>(
        &mut self,
        span: Span,
        run: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let root = self.type_work.is_none();
        if root {
            self.type_work = Some(Work {
                logical: Budget {
                    root: span,
                    ..Budget::default()
                },
                ..Work::default()
            });
        }
        let result = match &self.type_work.as_ref().unwrap().logical.failure {
            Some(error) => Err(error.clone()),
            None => run(self),
        };
        let result = match &self.type_work.as_ref().unwrap().logical.failure {
            Some(error) => Err(error.clone()),
            None => result,
        };
        if root {
            self.type_work = None;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn required_roots_share_work_and_restore_state_after_errors() {
        let mut checker = Checker::new();
        let span = Span::new(3, 7);
        let error = checker
            .required_root(span, |checker| {
                checker.type_work.as_mut().unwrap().spend(span)?;
                let result: Result<()> = checker.required_root(span, |checker| {
                    checker.type_work.as_mut().unwrap().spend(span)?;
                    Err(Checker::error("E107", "original failure", span))
                });
                assert_eq!(checker.type_work.as_ref().unwrap().visits, 2);
                result
            })
            .unwrap_err();
        assert_eq!(error.code, "E107");
        assert_eq!(error.span, span);
        assert!(checker.type_work.is_none());
        checker
            .required_root(span, |checker| {
                assert_eq!(checker.type_work.as_ref().unwrap().visits, 0);
                checker.type_work.as_mut().unwrap().spend(span)
            })
            .unwrap();
        assert!(checker.type_work.is_none());
    }
    #[test]
    pub(crate) fn logical_type_materialization_shares_origin_and_counts_repeated_nodes() {
        let span = Span::new(10, 40);
        let inner = Span::new(20, 30);
        let ty = Type::List {
            element: Box::new(Type::Bool),
            capacity: 2,
        };
        let mut checker = Checker::new();
        checker
            .required_root(span, |checker| {
                checker.type_work.as_mut().unwrap().materialize(&ty, span)?;
                checker.required_root(inner, |checker| {
                    checker.type_work.as_mut().unwrap().materialize(&ty, inner)
                })?;
                let budget = &checker.type_work.as_ref().unwrap().logical;
                assert_eq!(budget.root, span);
                assert_eq!((budget.steps, budget.types), (4, 4));
                Ok(())
            })
            .unwrap();
        assert!(checker.type_work.is_none());
    }

    #[test]
    pub(crate) fn logical_type_failure_cannot_be_swallowed_and_resets_next_root() {
        let span = Span::new(10, 40);
        let mut checker = Checker::new();
        let error = checker
            .required_root(span, |checker| {
                checker.type_work.as_mut().unwrap().logical.types =
                    crate::check::required::MAX_TYPES;
                let error = checker.type_work.as_mut().unwrap().node(span).unwrap_err();
                assert_eq!(error.code, "E220");
                let nested: Result<()> =
                    checker.required_root(span, |_| panic!("failed root ran again"));
                assert_eq!(nested.unwrap_err().code, "E220");
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error.span, span);
        assert!(checker.type_work.is_none());
        checker
            .required_root(Span::new(50, 60), |checker| {
                let work = checker.type_work.as_mut().unwrap();
                assert_eq!((work.logical.steps, work.logical.types), (0, 0));
                work.node(span)
            })
            .unwrap();
        assert!(checker.type_work.is_none());
        let mut work = Work {
            nodes: MAX_NODES,
            ..Work::default()
        };
        assert_eq!(work.node(span).unwrap_err().code, "B001");
        assert_eq!(work.logical.types, 0);
    }
    #[test]
    pub(crate) fn logical_roots_cover_real_metatype_bindings_without_resetting_nested_work() {
        let source = "kind<Type>:<int32>";
        let block = crate::parser::parse(source).unwrap();
        let crate::ast::StmtKind::Bind { value, ty, .. } = &block.stmts[0].kind else {
            panic!()
        };
        let span = Span::new(100, 140);
        let mut checker = Checker::new();
        let error = checker
            .required_root(span, |checker| {
                checker.type_work.as_mut().unwrap().logical.types =
                    crate::check::required::MAX_TYPES - 2;
                checker.meta_binding(value, ty.as_ref().unwrap())?;
                assert_eq!(
                    checker.type_work.as_ref().unwrap().logical.types,
                    crate::check::required::MAX_TYPES
                );
                checker.meta_binding(value, ty.as_ref().unwrap())
            })
            .err()
            .unwrap();
        assert_eq!(error.code, "E220");
        assert_eq!(error.span, span);
        assert!(checker.type_work.is_none());
        checker.meta_binding(value, ty.as_ref().unwrap()).unwrap();
        assert!(checker.type_work.is_none());
    }

    #[test]
    pub(crate) fn logical_type_charges_skip_inactive_construction_and_restore_source_failures() {
        let mut costs = Vec::new();
        for source in [
            "kind<Type>:{-><int32>}",
            "kind<Type>:{|false|unused:<int32[4]>;-><int32>}",
        ] {
            let block = crate::parser::parse(source).unwrap();
            let crate::ast::StmtKind::Bind { value, ty, .. } = &block.stmts[0].kind else {
                panic!()
            };
            let mut checker = Checker::new();
            let cost = checker
                .required_root(value.span, |checker| {
                    checker.meta_binding(value, ty.as_ref().unwrap())?;
                    let budget = &checker.type_work.as_ref().unwrap().logical;
                    assert_eq!(budget.root, value.span);
                    Ok((budget.steps, budget.types))
                })
                .unwrap();
            costs.push(cost);
        }
        assert_eq!(costs[0], costs[1]);
        let source = "kind<Type>:{bad:1/0;-><int32>}";
        let block = crate::parser::parse(source).unwrap();
        let crate::ast::StmtKind::Bind { value, ty, .. } = &block.stmts[0].kind else {
            panic!()
        };
        let mut checker = Checker::new();
        let scopes = checker.scopes.len();
        let error = checker
            .meta_binding(value, ty.as_ref().unwrap())
            .err()
            .unwrap();
        assert_eq!(error.code, "E107");
        assert_eq!(&source[error.span.start..error.span.end], "1/0");
        assert_eq!(checker.scopes.len(), scopes);
        assert!(checker.type_work.is_none());
    }
}
