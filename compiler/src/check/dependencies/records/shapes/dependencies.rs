use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn shaped_containers_follow_later_marks_and_control_queries() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x};p:@\"proof\"",
    );
    let wide = id(&checker, "wide");
    let x = id(&checker, "x");
    for value in checker
        .record_shapes
        .get_mut(&wide)
        .unwrap()
        .entries
        .values_mut()
    {
        value.origins = Origins {
            roots: BTreeSet::from([x]),
            complete: true,
        };
    }
    assert!(!checker.derived_local(wide));
    checker.mark_derived(x);
    assert!(checker.derived_local(wide));
    statements(&mut checker, "copy:wide;|copy<A>|q:p.can_copy<uint32>()");
    assert!(checker.derived_local(id(&checker, "copy")));
    assert!(checker.queries.last().unwrap().control);
    let parsed = crate::parser::parse("|wide<A>|out:wide.r").unwrap();
    assert_eq!(checker.stmt(&parsed.stmts[0]).unwrap_err().code, "E201");
}

#[test]
pub(crate) fn shaped_carrier_traversal_respects_prefixes_and_cycles() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "x:=false;a:&x;row:{->left:{->r:&a};->right:{->r:&a}};view:&row",
    );
    let row = id(&checker, "row");
    let view = id(&checker, "view");
    let x = id(&checker, "x");
    let a = id(&checker, "a");
    checker.record_cells.remove(&row);
    let span = Span::new(1, 2);
    let key = ShapeKey::new(&[1, 0], &[(1, &Type::Bool)], &mut checker.flow, span).unwrap();
    let snapshot = Snapshot {
        cells: Cells {
            places: BTreeSet::from([(row, vec![]), (a, vec![])]),
            complete: true,
        },
        ..Snapshot::default()
    };
    let mut shapes = Shapes::default();
    shapes
        .insert(key, snapshot, &mut checker.flow, span)
        .unwrap();
    checker.record_shapes.insert(row, shapes);
    assert!(!checker.derived_local(view));
    checker.mark_derived(x);
    assert!(checker.derived_local(row));
    assert!(checker.derived_local(view));
    checker.reference_cells.get_mut(&view).unwrap().places = BTreeSet::from([(row, vec![0])]);
    assert!(!checker.derived_local(view));
}
