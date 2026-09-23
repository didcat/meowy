use super::{carriers::id, tests::statements};
use crate::check::Checker;
use std::collections::BTreeSet;

#[test]
pub(crate) fn emitted_carriers_and_deeper_aliases_keep_external_pointees() {
    for source in [
        "x:=false;r:&x;row:{->cell:&r;->copy:*cell};out:row.copy",
        "x:=false;r:&x;cell:&r;row:{->outer:&cell;->copy:**outer};out:row.copy",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let out = id(&checker, "out");
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([0]));
        assert!(checker.pointees[&out].complete);
        checker.mark_derived(0);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn emitted_carrier_retargets_keep_ordinary_copy_snapshots() {
    let source = "x:=false;y:=true;r:&x;s:&y;row:{->cell:=&r;old:cell;cell=&s;->before:*old;->after:*cell};a:row.before;b:row.after";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert_eq!(
        checker.pointees[&id(&checker, "a")].roots,
        BTreeSet::from([0])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "b")].roots,
        BTreeSet::from([0, 1])
    );
}

#[test]
pub(crate) fn sibling_emitted_carriers_share_canonical_cell_sets() {
    let source = "x:=false;y:=true;r:&x;s:&y;c:=false;row:'out{|c|{'out->cell:=&r};|!c|{'out->cell:=&s;cell=&r;copy:*cell}}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let aliases = checker
        .proofs
        .aliases
        .iter()
        .filter(|(_, alias)| alias.field == "cell")
        .collect::<Vec<_>>();
    assert_eq!(aliases.len(), 2);
    assert_eq!(aliases[0].1.root, aliases[1].1.root);
    let root = aliases[0].1.root;
    assert_eq!(checker.reference_cells[&root].places.len(), 2);
    assert!(!checker.reference_cells.contains_key(aliases[1].0));
    assert!(
        checker
            .pointees
            .values()
            .any(|origins| origins.complete && origins.roots == BTreeSet::from([0, 1]))
    );
}

#[test]
pub(crate) fn unknown_emitted_cells_and_completed_record_carriers_stay_incomplete() {
    for source in [
        "x:=false;r:&x;row:{->cell:{->&r};->copy:*cell};out:row.copy",
        "x:=false;r:&x;row:{->cell:&r};cell:row.cell;out:*cell",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        assert!(!checker.pointees[&id(&checker, "out")].complete);
    }
}
