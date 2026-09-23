use super::*;
use crate::check::dependencies::{records::writes::id, tests::statements};
use std::collections::{BTreeMap, BTreeSet};

#[test]
pub(crate) fn seeded_record_cell_locations_feed_copies_and_dependency_reads() {
    let mut checker = Checker::new();
    statements(&mut checker, "x:=false;r:&x;row:{->cell:&r}");
    let row = id(&checker, "row");
    checker.record_cells.insert(
        row,
        BTreeMap::from([(
            vec![0],
            Cells {
                places: BTreeSet::from([(id(&checker, "r"), vec![])]),
                complete: true,
            },
        )]),
    );
    statements(&mut checker, "cell:row.cell;copy:*cell");
    assert_eq!(
        checker.pointees[&id(&checker, "copy")].roots,
        BTreeSet::from([0])
    );
    checker.mark_derived(0);
    assert!(checker.derived_local(row));
    assert!(checker.derived_local(id(&checker, "cell")));
}

#[test]
pub(crate) fn inline_record_carrier_fields_use_named_source_locations() {
    let source = "x:=false;r:&x;cell:({->cell:&r}).cell;copy:*cell";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert_eq!(
        checker.pointees[&id(&checker, "copy")].roots,
        BTreeSet::from([0])
    );
    assert!(checker.pointees[&id(&checker, "copy")].complete);
}

#[test]
pub(crate) fn record_path_classification_separates_pointees_and_carrier_cells() {
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, "x:=false;r:&x;row:{->a:&x;->b:&r}");
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    assert_eq!(checker.record_paths(value).unwrap(), vec![vec![0]]);
    assert_eq!(
        checker.record_paths_for(value, true).unwrap(),
        vec![vec![1]]
    );
}
