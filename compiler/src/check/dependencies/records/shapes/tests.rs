use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};
use std::collections::BTreeSet;

pub(super) fn snapshot(root: usize) -> Snapshot {
    Snapshot {
        origins: Origins {
            roots: BTreeSet::from([root]),
            complete: true,
        },
        cells: Cells {
            places: BTreeSet::from([(root, vec![])]),
            complete: true,
        },
    }
}

#[test]
pub(crate) fn shape_snapshots_keep_distinct_types_missing_entries_and_copies() {
    let mut flow = Flow::new();
    let span = Span::new(1, 2);
    let a = ShapeKey::new(&[0], &[(0, &Type::Bool)], &mut flow, span).unwrap();
    let b = ShapeKey::new(&[0], &[(0, &Type::Null)], &mut flow, span).unwrap();
    let mut shapes = Shapes::default();
    shapes
        .insert(a.clone(), snapshot(1), &mut flow, span)
        .unwrap();
    assert!(shapes.get(&b).is_none());
    shapes
        .insert(b.clone(), Snapshot::default(), &mut flow, span)
        .unwrap();
    let copy = shapes.clone();
    shapes
        .insert(a.clone(), snapshot(2), &mut flow, span)
        .unwrap();
    assert_eq!(copy.get(&a).unwrap().origins.roots, BTreeSet::from([1]));
    assert_eq!(
        shapes.get(&a).unwrap().cells.places,
        BTreeSet::from([(2, vec![])])
    );
    assert!(!shapes.get(&b).unwrap().origins.complete);
    assert!(!shapes.get(&b).unwrap().cells.complete);
}

#[test]
pub(crate) fn shape_snapshot_limits_preserve_existing_entries() {
    let mut flow = Flow::new();
    let span = Span::new(3, 4);
    let mut shapes = Shapes::default();
    for index in 0..MAX_FIELDS {
        let key = ShapeKey::new(&[index], &[(0, &Type::Bool)], &mut flow, span).unwrap();
        shapes
            .insert(key, snapshot(index), &mut flow, span)
            .unwrap();
    }
    let key = ShapeKey::new(&[0], &[(0, &Type::Null)], &mut flow, span).unwrap();
    assert_eq!(
        shapes
            .insert(key, snapshot(999), &mut flow, span)
            .unwrap_err()
            .code,
        "B001"
    );
    let key = ShapeKey::new(&[0], &[(0, &Type::Bool)], &mut flow, span).unwrap();
    let mut oversized = snapshot(999);
    oversized.origins.roots = (0..MAX_ROOTS + 1).collect();
    assert!(
        shapes
            .insert(key.clone(), oversized, &mut flow, span)
            .is_err()
    );
    let mut oversized = snapshot(999);
    oversized.cells.places = (0..MAX_ROOTS + 1).map(|id| (id, vec![])).collect();
    assert!(
        shapes
            .insert(key.clone(), oversized, &mut flow, span)
            .is_err()
    );
    assert!(!flow.spend(usize::MAX));
    assert!(
        shapes
            .insert(key.clone(), snapshot(999), &mut flow, span)
            .is_err()
    );
    assert_eq!(shapes.get(&key).unwrap().origins.roots, BTreeSet::from([0]));
    assert_eq!(shapes.entries.len(), MAX_FIELDS);
}

#[test]
pub(crate) fn shape_keys_bound_paths_types_and_selection_order() {
    let span = Span::new(5, 6);
    for (fields, variants) in [
        (vec![0; MAX_DEPTH], vec![(0, &Type::Bool)]),
        (vec![MAX_FIELDS], vec![]),
        (vec![0], vec![(2, &Type::Bool)]),
        (vec![0], vec![(1, &Type::Bool), (0, &Type::Null)]),
    ] {
        assert!(ShapeKey::new(&fields, &variants, &mut Flow::new(), span).is_err());
    }
    let ty = Type::Union(vec![Type::Bool; MAX_FIELDS * MAX_DEPTH + 1]);
    assert!(ShapeKey::new(&[0], &[(0, &ty)], &mut Flow::new(), span).is_err());
}

#[test]
pub(crate) fn mutable_union_source_layouts_register_only_incomplete_snapshots() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:={->r:&x};copy:wide;|copy<A>|out:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    for name in ["wide", "copy"] {
        let shapes = &checker.record_shapes[&id(&checker, name)];
        assert_eq!(shapes.entries.len(), 2);
        assert!(
            shapes
                .entries
                .values()
                .all(|value| !value.origins.complete && !value.cells.complete)
        );
    }
    assert!(!checker.pointees[&(checker.locals.len() - 1)].complete);
}
