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

#[test]
pub(crate) fn completed_carrier_fields_survive_copies_composition_and_nullable_records() {
    for source in [
        "x:=false;r:&x;row:{->cell:&r};copy:row;cell:copy.cell;out:*cell",
        "x:=false;r:&x;row:{->inner:{->cell:&r}};copy:{->row};out:*(copy.inner.cell)",
        "<R>:<&boolean>;<Box>:<{cell<&R>}>;x:=false;r:&x;row<Box><null>:{->cell:&r};|row<Box>|out:*(row.cell)",
        "x:=false;r:&x;row:{->cell:&r};outer:&(row.cell);out:**outer",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let out = checker.locals.len() - 1;
        assert_eq!(
            checker.pointees[&out].roots,
            BTreeSet::from([0]),
            "{source}"
        );
        assert!(checker.pointees[&out].complete);
        checker.mark_derived(0);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn carrier_field_and_subrecord_writes_preserve_prior_snapshots() {
    for update in [
        "row.inner.cell=&s",
        "row.inner={->cell:=&s}",
        "|condition|row.inner={->cell:=&s}",
    ] {
        let source = format!(
            "x:=false;y:=true;r:&x;s:&y;condition:=false;row:{{->inner:={{->cell:=&r}}}};old:row;{update};out:*(row.inner.cell);prior:*(old.inner.cell)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        assert_eq!(
            checker.pointees[&id(&checker, "out")].roots,
            BTreeSet::from([0, 1])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "prior")].roots,
            BTreeSet::from([0])
        );
    }
}

#[test]
pub(crate) fn carrier_field_updates_keep_unknown_sources_and_ordinary_errors() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "x:=false;r:&x;row:{->cell:=&r};row.cell={->&r};out:*(row.cell)",
    );
    assert!(!checker.pointees[&id(&checker, "out")].complete);
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([0])
    );
    let source = "x:=false;r:&x;row:{->cell:&r};row.cell=7";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E305");
}

#[test]
pub(crate) fn record_cell_capacity_errors_preserve_existing_snapshots() {
    let mut checker = Checker::new();
    let prior = Cells {
        places: (10..10 + MAX_ROOTS).map(|root| (root, vec![])).collect(),
        complete: true,
    };
    checker
        .record_cells
        .insert(0, BTreeMap::from([(vec![0], prior)]));
    let incoming = Cells {
        places: BTreeSet::from([(10 + MAX_ROOTS, vec![])]),
        complete: true,
    };
    let span = crate::ast::Span::new(1, 2);
    let error = checker
        .merge_record_cells(0, &[0], incoming, true, span)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("cell capacity"));
    assert_eq!(error.span, span);
    assert_eq!(checker.record_cells[&0][&vec![0]].places.len(), MAX_ROOTS);
    assert!(checker.record_cells[&0][&vec![0]].complete);
    let error = checker
        .merge_record_cells(0, &vec![0; 33], Cells::default(), true, span)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert_eq!(checker.record_cells[&0].len(), 1);
}
