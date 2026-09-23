use super::{Checker, carriers::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn borrowed_record_locations_survive_copies_and_nested_cells() {
    for tail in [
        "p:&a;view:p",
        "p:&a;c:&p;view:*c",
        "row:{->view:&a};view:row.view",
        "row:{->inner:{->view:&a}};view:row.inner.view",
    ] {
        let source = format!("x:=false;a:{{->r:&x}};{tail}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let view = id(&checker, "view");
        let cells = &checker.reference_cells[&view];
        assert!(cells.complete, "{tail}");
        assert_eq!(
            cells.places,
            BTreeSet::from([(id(&checker, "a"), vec![])]),
            "{tail}"
        );
        checker.mark_derived(id(&checker, "x"));
        assert!(checker.derived_local(view), "{tail}");
    }
}

#[test]
pub(crate) fn retargeted_record_views_preserve_old_snapshots_and_unknowns() {
    for (rhs, complete) in [("&b", true), ("{->&b}", false)] {
        let source = format!("x:=false;y:=true;a:{{->r:&x}};b:{{->r:&y}};p:=&a;copy:p;p={rhs}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let a = id(&checker, "a");
        let p = id(&checker, "p");
        let copy = id(&checker, "copy");
        assert_eq!(checker.reference_cells[&p].complete, complete);
        let mut places = BTreeSet::from([(a, vec![])]);
        if complete {
            places.insert((id(&checker, "b"), vec![]));
        }
        assert_eq!(checker.reference_cells[&p].places, places);
        assert_eq!(
            checker.reference_cells[&copy].places,
            BTreeSet::from([(a, vec![])])
        );
        checker.mark_derived(id(&checker, "y"));
        assert_eq!(checker.derived_local(p), complete);
        assert!(!checker.derived_local(copy));
    }
}

#[test]
pub(crate) fn subrecord_locations_exclude_unrelated_sibling_dependencies() {
    let source = "x:=false;y:=true;row:{->left:{->r:&x};->right:{->r:&y}};view:row.&left";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let view = id(&checker, "view");
    assert_eq!(
        checker.reference_cells[&view].places,
        BTreeSet::from([(id(&checker, "row"), vec![0])])
    );
    checker.mark_derived(id(&checker, "y"));
    assert!(!checker.derived_local(view));
    checker.mark_derived(id(&checker, "x"));
    assert!(checker.derived_local(view));
}

#[test]
pub(crate) fn borrowed_records_follow_nested_carrier_dependencies() {
    let source = "x:=false;p:&x;row:{->inner:{->cell:&p}};view:&row";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    checker.mark_derived(id(&checker, "x"));
    assert!(checker.derived_local(id(&checker, "view")));
}
