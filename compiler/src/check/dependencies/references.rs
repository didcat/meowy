use crate::check::Checker;
use crate::hir::{Expr, ExprKind, Type};

use std::collections::BTreeSet;

#[derive(Clone, Default)]
pub(crate) struct Origins {
    pub(crate) roots: BTreeSet<usize>,
    pub(crate) complete: bool,
}

#[derive(Clone, Default)]
pub(crate) struct Cells {
    pub(crate) places: BTreeSet<(usize, Vec<usize>)>,
    pub(crate) complete: bool,
}

pub(crate) const MAX_ROOTS: usize = 256;
pub(crate) const MAX_CELL_DEPTH: usize = 64;

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

    pub(crate) fn reference_cell(&mut self, expr: &Expr) -> crate::check::Result<Cells> {
        let mut value = expr;
        let mut depth = 0;
        let mut cells = loop {
            self.origin_visit(expr)?;
            match &value.kind {
                ExprKind::Borrow(place) => {
                    break Cells {
                        places: BTreeSet::from([(place.root, place.fields.clone())]),
                        complete: true,
                    };
                }
                ExprKind::TemporaryBorrow { id, .. } => {
                    break Cells {
                        places: BTreeSet::from([(*id, Vec::new())]),
                        complete: true,
                    };
                }
                ExprKind::Local(id) => {
                    break self
                        .reference_cells
                        .get(&self.origin_id(*id))
                        .cloned()
                        .unwrap_or_default();
                }
                ExprKind::Field { value, index } => {
                    break self.record_source_cells(value, &[*index])?;
                }
                ExprKind::Deref(inner) => {
                    depth += 1;
                    if depth > MAX_CELL_DEPTH {
                        return Err(crate::diagnostic::Diagnostic::unsupported(
                            "proof reference cell depth exhausted",
                            expr.span,
                        ));
                    }
                    value = inner;
                }
                ExprKind::Reborrow {
                    value: inner,
                    fields,
                    ..
                } if fields.is_empty() => value = inner,
                _ => break Cells::default(),
            }
        };
        for _ in 0..depth {
            cells = self.expand_reference_cells(cells, expr)?;
        }
        Ok(cells)
    }

    pub(crate) fn expand_reference_cells(
        &mut self,
        cells: Cells,
        expr: &Expr,
    ) -> crate::check::Result<Cells> {
        self.origin_visit(expr)?;
        let mut next = Cells {
            places: BTreeSet::new(),
            complete: cells.complete,
        };
        for (root, path) in cells.places {
            self.origin_visit(expr)?;
            let work = self
                .stored_cells(root, &path)
                .map_or(0, |source| source.places.len());
            if !self.flow.spend(work) {
                return Err(crate::diagnostic::Diagnostic::unsupported(
                    "proof reference cell budget exhausted",
                    expr.span,
                ));
            }
            let source = self.stored_cells(root, &path);
            next.complete &= source.is_some_and(|source| source.complete);
            if let Some(source) = source {
                next.places.extend(source.places.iter().cloned());
            }
            if next.places.len() > MAX_ROOTS {
                return Err(crate::diagnostic::Diagnostic::unsupported(
                    "proof reference cell capacity exhausted",
                    expr.span,
                ));
            }
        }
        Ok(next)
    }

    pub(crate) fn origin_visit(&mut self, expr: &Expr) -> crate::check::Result<()> {
        if !self.flow.spend(1) {
            return Err(crate::diagnostic::Diagnostic::unsupported(
                "proof reference origin traversal budget exhausted",
                expr.span,
            ));
        }
        Ok(())
    }

    pub(crate) fn cell_origins(&self, root: usize, path: &[usize]) -> Option<&Origins> {
        if path.is_empty() {
            self.pointees.get(&self.origin_id(root))
        } else {
            self.record_pointees
                .get(&root)
                .and_then(|fields| fields.get(path))
        }
    }

    pub(crate) fn origin_carrier(
        &mut self,
        ty: &Type,
        span: crate::ast::Span,
    ) -> crate::check::Result<bool> {
        let mut ty = ty.pointee();
        let mut depth = 0;
        loop {
            let Some(inner) = ty else { return Ok(false) };
            if !self.flow.spend(1) {
                return Err(crate::diagnostic::Diagnostic::unsupported(
                    "proof reference origin traversal budget exhausted",
                    span,
                ));
            }
            if Self::origin_reference(inner) {
                return Ok(true);
            }
            depth += 1;
            if depth > MAX_CELL_DEPTH {
                return Err(crate::diagnostic::Diagnostic::unsupported(
                    "proof reference cell depth exhausted",
                    span,
                ));
            }
            ty = inner.pointee();
        }
    }

    pub(crate) fn stored_cells(&self, root: usize, path: &[usize]) -> Option<&Cells> {
        if path.is_empty() {
            self.reference_cells.get(&self.origin_id(root))
        } else {
            self.record_cells
                .get(&root)
                .and_then(|fields| fields.get(path))
        }
    }

    pub(crate) fn track_reference_cell(
        &mut self,
        id: usize,
        value: &Expr,
        merge: bool,
    ) -> crate::check::Result<()> {
        if !self.origin_carrier(&value.ty, value.span)? {
            return Ok(());
        }
        let cells = self.reference_cell(value)?;
        self.store_cells(self.origin_id(id), cells, merge, value.span)
    }

    pub(crate) fn store_cells(
        &mut self,
        id: usize,
        mut cells: Cells,
        merge: bool,
        span: crate::ast::Span,
    ) -> crate::check::Result<()> {
        let prior = merge.then(|| self.reference_cells.get(&id)).flatten();
        let work = cells
            .places
            .iter()
            .chain(prior.into_iter().flat_map(|cells| &cells.places))
            .map(|(_, path)| path.len() + 1)
            .sum::<usize>()
            + 1;
        if !self.flow.spend(work) {
            return Err(crate::diagnostic::Diagnostic::unsupported(
                "proof reference cell budget exhausted",
                span,
            ));
        }
        if let Some(prior) = prior {
            cells.complete &= prior.complete;
            cells.places.extend(prior.places.iter().cloned());
        } else if merge {
            cells.complete = false;
        }
        if cells.places.len() > MAX_ROOTS {
            return Err(crate::diagnostic::Diagnostic::unsupported(
                "proof reference cell capacity exhausted",
                span,
            ));
        }
        self.reference_cells.insert(id, cells);
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

    pub(crate) fn reference_origins(&mut self, expr: &Expr) -> crate::check::Result<Origins> {
        self.reference_origins_at(expr, 0)
    }

    pub(crate) fn reference_origins_at(
        &mut self,
        expr: &Expr,
        depth: usize,
    ) -> crate::check::Result<Origins> {
        if depth > super::calls::MAX_DEPTH {
            return Err(crate::diagnostic::Diagnostic::unsupported(
                "proof reference call depth exhausted",
                expr.span,
            ));
        }
        let mut value = expr;
        loop {
            self.origin_visit(expr)?;
            match &value.kind {
                ExprKind::Borrow(place) | ExprKind::ExclusivePath { place, .. } => {
                    return Ok(Origins {
                        roots: BTreeSet::from([place.root]),
                        complete: true,
                    });
                }
                ExprKind::TemporaryBorrow { id, .. } => {
                    return Ok(Origins {
                        roots: BTreeSet::from([*id]),
                        complete: true,
                    });
                }
                ExprKind::Call { args, .. } => {
                    return self.call_reference_origins(value, args, depth);
                }
                ExprKind::Deref(inner) => {
                    let cells = self.reference_cell(inner)?;
                    let mut origins = Origins {
                        roots: BTreeSet::new(),
                        complete: cells.complete,
                    };
                    for (root, path) in cells.places {
                        self.origin_visit(expr)?;
                        let source = self.cell_origins(root, &path);
                        origins.complete &= source.is_some_and(|source| source.complete);
                        if let Some(source) = source {
                            origins.roots.extend(&source.roots);
                        }
                        if origins.roots.len() > MAX_ROOTS {
                            return Err(crate::diagnostic::Diagnostic::unsupported(
                                "proof reference origin capacity exhausted",
                                expr.span,
                            ));
                        }
                    }
                    return Ok(origins);
                }
                ExprKind::Local(id) => {
                    return Ok(self
                        .pointees
                        .get(&self.origin_id(*id))
                        .cloned()
                        .unwrap_or_default());
                }
                ExprKind::Field { value, index } => return Ok(self.field_origins(value, *index)),
                ExprKind::Reborrow { value: inner, .. }
                | ExprKind::ElementBorrow { value: inner, .. }
                | ExprKind::Coerce { value: inner } => value = inner,
                _ => return Ok(Origins::default()),
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
        let origins = self.reference_origins(value)?;
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
