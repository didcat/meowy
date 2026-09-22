use crate::check::Checker;
use crate::hir::{Expr, ExprKind, Type};

use std::collections::BTreeSet;

#[derive(Clone, Default)]
pub(crate) struct Origins {
    pub(crate) roots: BTreeSet<usize>,
    pub(crate) complete: bool,
}

pub(crate) const MAX_ROOTS: usize = 256;

impl Checker {
    pub(crate) fn reference_origins(&self, expr: &Expr) -> Origins {
        let mut value = expr;
        loop {
            match &value.kind {
                ExprKind::Borrow(place) | ExprKind::ExclusivePath { place, .. } => {
                    return Origins {
                        roots: BTreeSet::from([place.root]),
                        complete: true,
                    };
                }
                ExprKind::Local(id) => return self.pointees.get(id).cloned().unwrap_or_default(),
                ExprKind::Reborrow { value: inner, .. } | ExprKind::Coerce { value: inner } => {
                    value = inner
                }
                _ => return Origins::default(),
            }
        }
    }

    pub(crate) fn track_reference(
        &mut self,
        id: usize,
        value: &Expr,
        merge: bool,
    ) -> crate::check::Result<()> {
        if !matches!(
            value.ty.pointee(),
            Some(Type::Bool | Type::Int { .. } | Type::Float { .. })
        ) {
            return Ok(());
        }
        let mut origins = self.reference_origins(value);
        let prior = merge.then(|| self.pointees.get(&id)).flatten();
        let work = origins.roots.len() + prior.map_or(0, |prior| prior.roots.len()) + 1;
        if !self.flow.spend(work) {
            return Err(crate::diagnostic::Diagnostic::unsupported(
                "proof reference origin budget exhausted",
                value.span,
            ));
        }
        if let Some(prior) = prior {
            origins.complete &= prior.complete;
            origins.roots.extend(&prior.roots);
        } else if merge {
            origins.complete = false;
        }
        if origins.roots.len() > MAX_ROOTS {
            return Err(crate::diagnostic::Diagnostic::unsupported(
                "proof reference origin capacity exhausted",
                value.span,
            ));
        }
        self.pointees.insert(id, origins);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::dependencies::tests::statements;

    #[test]
    pub(crate) fn scalar_reference_copies_observe_later_owner_marks() {
        let mut checker = Checker::new();
        statements(
            &mut checker,
            "x:=false;r:&x;copy:r;other:=false;plain:&other",
        );
        assert_eq!(checker.pointees[&1].roots, BTreeSet::from([0]));
        assert_eq!(checker.pointees[&2].roots, BTreeSet::from([0]));
        assert!(!checker.derived_local(2));
        checker.mark_derived(0);
        assert!(checker.derived_local(1));
        assert!(checker.derived_local(2));
        assert!(!checker.derived_local(4));
        statements(
            &mut checker,
            "flag:*copy;p:@\"proof\";|flag|r:p.can_copy<uint32>()",
        );
        assert!(checker.queries[0].control);
    }

    #[test]
    pub(crate) fn scalar_reborrows_preserve_whole_owner_identity() {
        for source in [
            "x:=false;r:&!x;copy:&!*r",
            "x:=false;r:&!x;copy<&boolean>:r",
        ] {
            let mut checker = Checker::new();
            statements(&mut checker, source);
            assert_eq!(checker.pointees[&1].roots, BTreeSet::from([0]));
            assert_eq!(checker.pointees[&2].roots, BTreeSet::from([0]));
            checker.mark_derived(0);
            assert!(checker.derived_local(2));
        }
    }

    #[test]
    pub(crate) fn unsupported_reference_origins_are_not_invented() {
        for source in ["x:=false;r:{->&x};copy:r", "x:{->n:7};r:&x;copy:r"] {
            let mut checker = Checker::new();
            statements(&mut checker, source);
            assert!(
                checker
                    .pointees
                    .values()
                    .all(|origins| !origins.complete && origins.roots.is_empty())
            );
        }
    }
}

#[cfg(test)]
mod origins {
    use super::*;
    use crate::ast::Span;

    pub(crate) fn reference(id: usize) -> Expr {
        Expr {
            kind: ExprKind::Local(id),
            ty: Type::Reference(Box::new(Type::Bool)),
            span: Span::new(1, 2),
        }
    }

    #[test]
    pub(crate) fn origin_merges_keep_snapshots_and_incomplete_sources() {
        let mut checker = Checker::new();
        checker.pointees.insert(
            0,
            Origins {
                roots: BTreeSet::from([10]),
                complete: true,
            },
        );
        checker.pointees.insert(
            1,
            Origins {
                roots: BTreeSet::from([11]),
                complete: true,
            },
        );
        checker.track_reference(2, &reference(0), false).unwrap();
        checker.track_reference(0, &reference(1), true).unwrap();
        assert_eq!(checker.pointees[&0].roots, BTreeSet::from([10, 11]));
        assert!(checker.pointees[&0].complete);
        assert_eq!(checker.pointees[&2].roots, BTreeSet::from([10]));
        checker.track_reference(0, &reference(3), true).unwrap();
        assert!(!checker.pointees[&0].complete);
        assert_eq!(checker.pointees[&0].roots, BTreeSet::from([10, 11]));
        checker.mark_derived(11);
        assert!(checker.derived_local(0));
        assert!(!checker.derived_local(2));
    }

    #[test]
    pub(crate) fn origin_capacity_keeps_previous_state_on_failure() {
        let mut checker = Checker::new();
        checker.pointees.insert(
            0,
            Origins {
                roots: (10..10 + MAX_ROOTS).collect(),
                complete: true,
            },
        );
        checker.pointees.insert(
            1,
            Origins {
                roots: BTreeSet::from([10 + MAX_ROOTS]),
                complete: true,
            },
        );
        checker.track_reference(0, &reference(0), true).unwrap();
        let error = checker.track_reference(0, &reference(1), true).unwrap_err();
        assert_eq!(error.code, "B001");
        assert_eq!(error.span, reference(1).span);
        assert!(error.message.contains("origin capacity"));
        assert_eq!(checker.pointees[&0].roots.len(), MAX_ROOTS);
        assert!(checker.pointees[&0].complete);
    }
}
