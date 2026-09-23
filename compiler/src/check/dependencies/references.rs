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
    pub(crate) fn temporary_storage(expr: &Expr) -> Option<usize> {
        let mut value = expr;
        loop {
            match &value.kind {
                ExprKind::TemporaryBorrow { id, .. } => return Some(*id),
                ExprKind::Reborrow {
                    value: inner,
                    fields,
                    ..
                } if fields.is_empty() => value = inner,
                _ => return None,
            }
        }
    }

    pub(crate) fn reference_cell(&self, expr: &Expr) -> Option<crate::hir::Place> {
        let mut value = expr;
        loop {
            match &value.kind {
                ExprKind::Borrow(place) => return Some(place.clone()),
                ExprKind::TemporaryBorrow { id, .. } => {
                    return Some(crate::hir::Place {
                        root: *id,
                        fields: Vec::new(),
                    });
                }
                ExprKind::Local(id) => return self.reference_cells.get(id).cloned(),
                ExprKind::Reborrow {
                    value: inner,
                    fields,
                    ..
                } if fields.is_empty() => value = inner,
                _ => return None,
            }
        }
    }

    pub(crate) fn cell_origins(&self, cell: &crate::hir::Place) -> Option<&Origins> {
        if cell.fields.is_empty() {
            self.pointees.get(&self.origin_id(cell.root))
        } else {
            self.record_pointees
                .get(&cell.root)
                .and_then(|fields| fields.get(&cell.fields))
        }
    }

    pub(crate) fn track_reference_cell(
        &mut self,
        id: usize,
        value: &Expr,
    ) -> crate::check::Result<()> {
        if !value.ty.pointee().is_some_and(Self::origin_reference) {
            return Ok(());
        }
        if let Some(cell) = self.reference_cell(value) {
            if !self.flow.spend(cell.fields.len() + 1) {
                return Err(crate::diagnostic::Diagnostic::unsupported(
                    "proof reference cell budget exhausted",
                    value.span,
                ));
            }
            self.reference_cells.insert(id, cell);
        }
        Ok(())
    }

    pub(crate) fn origin_reference(ty: &Type) -> bool {
        ty.pointee().is_some_and(|ty| {
            !ty.has_reference()
                && matches!(
                    ty,
                    Type::Bool
                        | Type::Int { .. }
                        | Type::Float { .. }
                        | Type::List { .. }
                        | Type::Record { .. }
                )
        })
    }

    pub(crate) fn origin_id(&self, id: usize) -> usize {
        self.proofs.aliases.get(&id).map_or(id, |alias| alias.root)
    }

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
                ExprKind::TemporaryBorrow { id, .. } => {
                    return Origins {
                        roots: BTreeSet::from([*id]),
                        complete: true,
                    };
                }
                ExprKind::Deref(inner) => {
                    let Some(cell) = self.reference_cell(inner) else {
                        return Origins::default();
                    };
                    return self.cell_origins(&cell).cloned().unwrap_or_default();
                }
                ExprKind::Local(id) => {
                    return self
                        .pointees
                        .get(&self.origin_id(*id))
                        .cloned()
                        .unwrap_or_default();
                }
                ExprKind::Field { value, index } => return self.field_origins(value, *index),
                ExprKind::Reborrow { value: inner, .. }
                | ExprKind::ElementBorrow { value: inner, .. }
                | ExprKind::Coerce { value: inner } => value = inner,
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
        if !Self::origin_reference(&value.ty) {
            return Ok(());
        }
        let origins = self.reference_origins(value);
        self.store_origins(self.origin_id(id), origins, merge, value.span)
    }

    pub(crate) fn store_origins(
        &mut self,
        id: usize,
        mut origins: Origins,
        merge: bool,
        span: crate::ast::Span,
    ) -> crate::check::Result<()> {
        let prior = merge.then(|| self.pointees.get(&id)).flatten();
        let work = origins.roots.len() + prior.map_or(0, |prior| prior.roots.len()) + 1;
        if !self.flow.spend(work) {
            return Err(crate::diagnostic::Diagnostic::unsupported(
                "proof reference origin budget exhausted",
                span,
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
                span,
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
        for source in ["x:=false;r:{->&x};copy:r", "n:7;x:{->r:&n};r:&x;copy:r"] {
            let mut checker = Checker::new();
            statements(&mut checker, source);
            assert!(
                checker
                    .pointees
                    .get(&(checker.locals.len() - 1))
                    .is_none_or(|origins| !origins.complete)
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
