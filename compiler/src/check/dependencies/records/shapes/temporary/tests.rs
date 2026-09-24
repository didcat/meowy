use super::*;
use crate::ast::Span;
use crate::check::dependencies::{carriers::id, temporaries::borrow, tests::statements};
use crate::hir::Type;
use std::collections::BTreeSet;

pub(super) fn reborrow(value: Expr, fields: Vec<usize>) -> Expr {
    Expr {
        ty: value.ty.clone(),
        span: value.span,
        kind: ExprKind::Reborrow {
            site: 0,
            value: Box::new(value),
            fields,
        },
    }
}

#[test]
pub(crate) fn direct_temporary_union_reads_recover_seeded_contents_not_cell_owners() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x}",
    );
    let wide = id(&checker, "wide");
    let ty = checker.locals[wide].clone();
    let view = borrow(
        &mut checker,
        Expr {
            kind: ExprKind::Local(wide),
            ty: ty.clone(),
            span: Span::new(1, 2),
        },
    );
    let cell = checker.shape_temporary(&view).unwrap().unwrap().root;
    checker
        .record_shapes
        .insert(cell, checker.record_shapes[&wide].clone());
    let key = checker.record_shapes[&cell]
        .entries
        .keys()
        .next()
        .unwrap()
        .clone();
    for view in [view.clone(), reborrow(view, vec![])] {
        let value = Expr {
            kind: ExprKind::Deref(Box::new(view)),
            ty: ty.clone(),
            span: Span::new(1, 2),
        };
        let snapshot = checker.record_shape_source(&value, &key).unwrap();
        assert!(snapshot.origins.complete);
        assert_eq!(snapshot.origins.roots, BTreeSet::from([id(&checker, "x")]));
        assert!(!snapshot.origins.roots.contains(&cell));
        let narrowed = Expr {
            ty: key.variants[0].1.clone(),
            span: value.span,
            kind: ExprKind::Coerce {
                value: Box::new(value),
            },
        };
        assert!(
            checker
                .record_shape_snapshot(&narrowed, &[0])
                .unwrap()
                .unwrap()
                .origins
                .complete
        );
    }
}

#[test]
pub(crate) fn temporary_shape_lookup_preserves_unknown_views_and_bounds() {
    let mut checker = Checker::new();
    let value = Expr {
        kind: ExprKind::Bool(false),
        ty: Type::Bool,
        span: Span::new(3, 4),
    };
    let view = borrow(&mut checker, value);
    let cell = checker.shape_temporary(&view).unwrap().unwrap().root;
    assert!(
        checker
            .shape_temporary(&reborrow(view.clone(), vec![0]))
            .unwrap()
            .is_none()
    );
    let named = Expr {
        kind: ExprKind::Local(0),
        ty: view.ty.clone(),
        span: view.span,
    };
    assert!(checker.shape_temporary(&named).unwrap().is_none());
    let mut chain = view;
    for _ in 0..MAX_DEPTH {
        chain = reborrow(chain, vec![]);
    }
    assert_eq!(
        checker
            .shape_temporary(&chain)
            .unwrap()
            .map(|place| place.root),
        Some(cell)
    );
    let chain = reborrow(chain, vec![]);
    let error = checker.shape_temporary(&chain).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("shape depth"));
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.shape_temporary(&named).is_err());
}

#[test]
pub(crate) fn projected_temporary_shapes_validate_paths_and_shift_narrowing_keys() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x};row:{->flag:false;->inner:{->item:wide}}",
    );
    let row = id(&checker, "row");
    let wide = id(&checker, "wide");
    let ty = checker.locals[wide].clone();
    let selected = checker.record_shapes[&wide]
        .entries
        .keys()
        .next()
        .unwrap()
        .variants[0]
        .1
        .clone();
    let source = Expr {
        kind: ExprKind::Local(row),
        ty: checker.locals[row].clone(),
        span: Span::new(5, 6),
    };
    let view = borrow(&mut checker, source);
    let mut projected = reborrow(view.clone(), vec![1, 0]);
    projected.ty = Type::Reference(Box::new(ty.clone()));
    let place = checker.shape_temporary(&projected).unwrap().unwrap();
    assert_eq!(place.fields, [1, 0]);
    let copied = Expr {
        kind: ExprKind::Deref(Box::new(projected)),
        ty,
        span: view.span,
    };
    let narrowed = Expr {
        kind: ExprKind::Coerce {
            value: Box::new(copied),
        },
        ty: selected,
        span: view.span,
    };
    let snapshot = checker
        .record_shape_snapshot(&narrowed, &[0])
        .unwrap()
        .unwrap();
    assert!(snapshot.origins.complete);
    assert_eq!(snapshot.origins.roots, BTreeSet::from([id(&checker, "x")]));
    for path in [vec![99], vec![1, 0, 0]] {
        assert!(
            checker
                .shape_temporary(&reborrow(view.clone(), path))
                .unwrap()
                .is_none()
        );
    }
    let error = checker
        .shape_temporary(&reborrow(view, vec![0; MAX_DEPTH + 1]))
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("path budget"));
}
