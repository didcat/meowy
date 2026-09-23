use super::{carriers::id, tests::statements};
use crate::check::Checker;
use std::collections::BTreeSet;

#[test]
pub(crate) fn direct_and_named_reference_cells_keep_external_pointees() {
    for source in [
        "x:=false;r:&x;copy:*(&r)",
        "x<boolean[1]>:=[false];view:&x;cell:&view;copy:*cell",
        "x:=false;r:&x;cell:&r;copy:*cell",
        "x:=false;r:&x;cell:&r;alias:cell;copy:*alias",
        "x:=false;r:&x;cell:&r;alias:&*cell;copy:*alias",
        "x:=false;row:{->r:&x};cell:&(row.r);copy:*cell",
        "x:=false;row:{->inner:{->r:&x}};cell:&(row.inner.r);copy:*cell",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let owner = id(&checker, "x");
        let copy = id(&checker, "copy");
        assert_eq!(
            checker.pointees[&copy].roots,
            BTreeSet::from([owner]),
            "{source}"
        );
        assert!(checker.pointees[&copy].complete);
        checker.mark_derived(owner);
        assert!(checker.derived_local(copy));
    }
}

#[test]
pub(crate) fn reference_cell_reads_preserve_earlier_value_snapshots() {
    let source = "x:=false;y:=true;r:=&x;old:*(&r);r=&y;cell:&r;copy:*cell";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert_eq!(
        checker.pointees[&id(&checker, "old")].roots,
        BTreeSet::from([0])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "copy")].roots,
        BTreeSet::from([0, 1])
    );
    checker.mark_derived(1);
    assert!(checker.derived_local(id(&checker, "cell")));
    assert!(!checker.derived_local(id(&checker, "old")));
}

#[test]
pub(crate) fn unsupported_cell_aliases_and_unknown_contents_stay_incomplete() {
    for source in [
        "x:=false;r:&x;cell:=&r;copy:*cell",
        "f<&boolean>:(v<&boolean>){->v};x:=false;r:f(&x);cell:&r;copy:*cell",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        assert!(!checker.pointees[&id(&checker, "copy")].complete);
    }
    let errors = crate::compile("x:=false;y:=true;r:=&x;cell:&r;r=&y;copy:*cell").unwrap_err();
    assert_eq!(errors[0].code, "E302");
}
