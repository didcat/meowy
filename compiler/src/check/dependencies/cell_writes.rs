use super::{carriers::id, tests::statements};
use crate::check::Checker;
use std::collections::BTreeSet;

#[test]
pub(crate) fn mutable_cell_retargets_preserve_carrier_and_value_snapshots() {
    for update in ["cell=&s", "|condition|cell=&s"] {
        let source = format!(
            "x:=false;y:=true;r:&x;s:&y;cell:=&r;old:cell;value:*cell;condition:=false;{update};copy:*cell;prior:*old"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        assert_eq!(
            checker.pointees[&id(&checker, "copy")].roots,
            BTreeSet::from([0, 1])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "prior")].roots,
            BTreeSet::from([0])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "value")].roots,
            BTreeSet::from([0])
        );
        assert!(checker.pointees[&id(&checker, "copy")].complete);
        checker.mark_derived(1);
        assert!(checker.derived_local(id(&checker, "cell")));
        assert!(!checker.derived_local(id(&checker, "old")));
    }
}

#[test]
pub(crate) fn retargeted_field_cells_keep_distinct_locations() {
    let source = "x:=false;y:=true;row:{->a:&x;->b:&y};cell:=&(row.a);cell=&(row.b);copy:*cell";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let row = id(&checker, "row");
    assert_eq!(
        checker.reference_cells[&id(&checker, "cell")].places,
        BTreeSet::from([(row, vec![0]), (row, vec![1])])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "copy")].roots,
        BTreeSet::from([0, 1])
    );
}

#[test]
pub(crate) fn unknown_retargets_retain_possible_cells_but_not_completeness() {
    for (body, owner) in [("cell:=&r;cell={->&s}", 0), ("cell:={->&r};cell=&s", 1)] {
        let source = format!("x:=false;y:=true;r:&x;s:&y;{body};copy:*cell");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "copy")];
        assert_eq!(origins.roots, BTreeSet::from([owner]));
        assert!(!origins.complete);
    }
}

#[test]
pub(crate) fn invalid_carrier_updates_keep_existing_cell_metadata() {
    for (binding, code) in [("cell:&r", "E305"), ("cell:=&r", "E207")] {
        let mut checker = Checker::new();
        statements(&mut checker, &format!("x:=false;r:&x;{binding}"));
        let block = crate::parser::parse("cell=7").unwrap();
        assert_eq!(checker.stmt(&block.stmts[0]).unwrap_err().code, code);
        let cells = &checker.reference_cells[&id(&checker, "cell")];
        assert!(cells.complete);
        assert_eq!(cells.places, BTreeSet::from([(id(&checker, "r"), vec![])]));
    }
}
