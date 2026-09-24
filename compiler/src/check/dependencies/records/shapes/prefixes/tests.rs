use super::*;
use crate::check::dependencies::{
    carriers::id, records::shapes::tests::snapshot, tests::statements,
};
use crate::hir::Type;
use std::collections::BTreeSet;

#[test]
pub(crate) fn owned_union_field_writes_merge_seeded_prefixes_and_preserve_siblings() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;z:=false;row:{->left<A><B>:={->r:&x};->right<A><B>:{->r:&z}}",
    );
    let row = id(&checker, "row");
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    let z = id(&checker, "z");
    for (key, value) in &mut checker.record_shapes.get_mut(&row).unwrap().entries {
        *value = snapshot(if key.fields[0] == 0 { x } else { z });
    }
    let old = checker.record_shapes[&row].clone();
    statements(&mut checker, "row.left={->r:&y}");
    for (key, value) in &checker.record_shapes[&row].entries {
        if key.fields[0] == 1 {
            assert_eq!(value.origins.roots, BTreeSet::from([z]));
            assert!(value.origins.complete);
        }
    }
    assert!(
        checker.record_shapes[&row]
            .snapshots(&[0])
            .any(|value| value.origins.roots == BTreeSet::from([x, y]))
    );
    assert!(
        old.snapshots(&[0])
            .all(|value| value.origins.roots == BTreeSet::from([x]))
    );
}

#[test]
pub(crate) fn shape_prefix_merges_shift_selections_and_keep_missing_inputs_incomplete() {
    let mut flow = Flow::new();
    let span = Span::new(1, 2);
    let mut next = Shapes::default();
    let key = ShapeKey::new(&[0], &[(0, &Type::Bool)], &mut flow, span).unwrap();
    next.insert(key, snapshot(7), &mut flow, span).unwrap();
    let merged = Shapes::default()
        .merged_prefix(&next, &[2, 1], &mut flow, span)
        .unwrap();
    let (key, value) = merged.entries.first_key_value().unwrap();
    assert_eq!(key.fields, [2, 1, 0]);
    assert_eq!(key.variants, [(2, Type::Bool)]);
    assert!(!value.origins.complete && !value.cells.complete);
    assert_eq!(value.origins.roots, BTreeSet::from([7]));
    let error = merged
        .merged_prefix(&next, &[2, 1, 0], &mut flow, span)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("crosses a union"));
    assert!(
        Shapes::default()
            .merged_prefix(&next, &[0; 32], &mut flow, span)
            .is_err()
    );
}

#[test]
pub(crate) fn shape_prefix_failures_preserve_existing_snapshots() {
    let mut checker = Checker::new();
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;row:{->item<A><B>:={->r:&x}};next<A><B>:{->r:&x}";
    let stmts = statements(&mut checker, source);
    let row = id(&checker, "row");
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let count = checker.record_shapes[&row].entries.len();
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.track_shape_prefix(row, &[0], value).is_err());
    assert_eq!(checker.record_shapes[&row].entries.len(), count);
}

#[test]
pub(crate) fn shape_prefix_updates_bound_total_entries_and_ignore_unrelated_paths() {
    let mut flow = Flow::new();
    let span = Span::new(3, 4);
    let mut prior = Shapes::default();
    for index in 1..256 {
        let key = ShapeKey::new(&[index], &[(0, &Type::Bool)], &mut flow, span).unwrap();
        prior.insert(key, snapshot(1), &mut flow, span).unwrap();
    }
    let mut next = Shapes::default();
    for index in 0..2 {
        let key = ShapeKey::new(&[index], &[(0, &Type::Bool)], &mut flow, span).unwrap();
        next.insert(key, snapshot(2), &mut flow, span).unwrap();
    }
    assert!(prior.merged_prefix(&next, &[0], &mut flow, span).is_err());
    assert_eq!(prior.entries.len(), 255);
    let mut checker = Checker::new();
    checker.record_shapes.insert(0, prior);
    let value = Expr {
        kind: crate::hir::ExprKind::Bool(false),
        ty: Type::Bool,
        span,
    };
    checker.track_shape_prefix(0, &[300], &value).unwrap();
    assert_eq!(checker.record_shapes[&0].entries.len(), 255);
}
