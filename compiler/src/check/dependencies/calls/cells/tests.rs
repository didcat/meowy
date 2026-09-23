use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn returned_shared_cells_preserve_locations_and_stored_owners() {
    for tail in [
        "r:f(&a);out:*r",
        "r:f(f(&a));copy:r;out:*copy",
        "out:*(f(&a))",
    ] {
        let source = format!("f<& &boolean>:(p<& &boolean>){{->p}};x:=false;a:&x;{tail}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let out = id(&checker, "out");
        assert!(checker.pointees[&out].complete);
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn returned_shared_cells_keep_all_exact_contract_candidates() {
    let source = "f<& &boolean>:(a<& &boolean>,b<& &boolean>,n<int32>){->a};x:=false;y:=true;a:&x;b:&y;r:f(&a,&b,7);out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "a"), vec![]), (id(&checker, "b"), vec![])])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
}

#[test]
pub(crate) fn unknown_returned_cell_candidates_keep_known_locations_incomplete() {
    let source = "f<& &boolean>:(a<& &boolean>,b<& &boolean>){->a};x:=false;y:=true;a:&x;b:&y;r:f(&a,{->&b});out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(!cells.complete);
    assert_eq!(cells.places, BTreeSet::from([(id(&checker, "a"), vec![])]));
    assert!(!checker.pointees[&id(&checker, "out")].complete);
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([id(&checker, "x")])
    );
}

#[test]
pub(crate) fn returned_cell_analysis_is_bounded_and_does_not_replay_calls() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "f<& &boolean>:(p<& &boolean>){->p};x:=false;a:&x;r:f(&a)",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    checker.reference_cell(value).unwrap();
    assert_eq!(checker.calls, calls);
    let error = checker
        .reference_cell_at(value, super::super::MAX_DEPTH + 1)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("call depth"));
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(checker.reference_cell(value).err().unwrap().code, "B001");
}

#[test]
pub(crate) fn returned_cell_lifetimes_and_unknown_nested_views_remain_checked() {
    let source = "f<& &boolean>:(p<& &boolean>){local:*p;->&local};x:=false;a:&x;r:f(&a)";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
    let source = "<N>:<{c<&boolean>}>;<R>:<{inner<&N>}>;f<& &boolean>:(p<&R>){view:p.inner;->view.&c};x:=false;a<N>:{->c:&x};row<R>:{->inner:{->&a}};r:f(&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.reference_cells[&id(&checker, "r")].complete);
}
