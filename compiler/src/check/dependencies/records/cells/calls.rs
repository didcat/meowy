use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn record_call_carrier_fields_keep_copied_and_nested_locations() {
    for source in [
        "<R>:<{c<& &boolean>}>;f<R>:(p<& &boolean>){->c:p};x:=false;a:&x;row:f(&a);out:*(row.c)",
        "<R>:<{c<& &boolean>}>;f<R>:(p<& &boolean>){->c:p};x:=false;a:&x;out:*((f(&a)).c)",
        "<R>:<{c<& &boolean>}>;f<R>:(p<& &boolean>){->c:p};x:=false;a:&x;row:f(&a);copy:row;out:*(copy.c)",
        "<R>:<{c<& &boolean>}>;<N>:<{inner<R>}>;f<N>:(p<& &boolean>){->inner:{->c:p}};x:=false;a:&x;row:f(&a);out:*(row.inner.c)",
        "<R>:<{c<& & &boolean>}>;f<R>:(p<& & &boolean>){->c:p};x:=false;a:&x;b:&a;row:f(&b);out:**(row.c)",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let x = id(&checker, "x");
        let out = id(&checker, "out");
        assert!(checker.pointees[&out].complete, "{source}");
        assert_eq!(
            checker.pointees[&out].roots,
            BTreeSet::from([x]),
            "{source}"
        );
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn record_call_view_fields_feed_later_origin_queries() {
    let source = "<R>:<{r<&boolean>}>;<W>:<{view<&R>}>;f<W>:(p<&R>){->view:p};read<&boolean>:(p<&R>){->p.r};x:=false;a<R>:{->r:&x};row:f(&a);out:read(row.view)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert_eq!(
        checker.record_cells[&id(&checker, "row")][&vec![0]].places,
        BTreeSet::from([(id(&checker, "a"), vec![])])
    );
    assert!(checker.pointees[&id(&checker, "out")].complete);
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([id(&checker, "x")])
    );
}

#[test]
pub(crate) fn record_carrier_results_keep_all_candidates_and_unknowns() {
    for (arg, complete, names) in [("&b", true, vec!["a", "b"]), ("{->&b}", false, vec!["a"])] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}>;f<R>:(a<& &boolean>,b<& &boolean>){{->c:a}};x:=false;y:=true;a:&x;b:&y;row:f(&a,{arg});out:*(row.c)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.record_cells[&id(&checker, "row")][&vec![0]];
        assert_eq!(cells.complete, complete);
        assert_eq!(
            cells.places,
            names
                .iter()
                .map(|name| (id(&checker, name), vec![]))
                .collect()
        );
        assert_eq!(checker.pointees[&id(&checker, "out")].complete, complete);
    }
}

#[test]
pub(crate) fn nested_record_carrier_calls_share_depth_and_keep_lifetime_errors() {
    let source = "<R>:<{c<& &boolean>}>;make<R>:(p<& &boolean>){->c:p};copy<R>:(p<R>){->c:p.c};x:=false;a:&x;row:copy(copy(make(&a)))";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    assert!(checker.record_cells[&id(&checker, "row")][&vec![0]].complete);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    let cells = checker.record_source_cells(value, &[0]).unwrap();
    assert_eq!(cells.places, BTreeSet::from([(id(&checker, "a"), vec![])]));
    assert_eq!(checker.calls, calls);
    assert!(!checker.record_source_cells(value, &[1]).unwrap().complete);
    let error = checker
        .record_source_cells_at(value, &[0], crate::check::dependencies::calls::MAX_DEPTH)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("record cell call depth"));
    let source =
        "<R>:<{c<& &boolean>}>;f<R>:(p<& &boolean>){local:*p;->c:&local};x:=false;a:&x;row:f(&a)";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn record_call_subrecord_projections_keep_carrier_snapshots() {
    for tail in [
        "out:*(f(&a).inner.c)",
        "row:f(&a).inner;out:*(row.c)",
        "row:g(f(&a).inner);out:*(row.c)",
    ] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}>;<N>:<{{inner<R>}}>;f<N>:(p<& &boolean>){{->inner:{{->c:p}}}};g<R>:(p<R>){{->c:p.c}};x:=false;a:&x;{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let out = id(&checker, "out");
        assert!(checker.pointees[&out].complete, "{tail}");
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn projected_carrier_calls_keep_unknowns_and_depth_limits() {
    for (arg, complete) in [("&a", true), ("{->&a}", false)] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}>;<N>:<{{inner<R>}}>;f<N>:(p<& &boolean>){{->inner:{{->c:p}}}};g<N>:(p<R>){{->inner:{{->c:p.c}}}};x:=false;a:&x;row:g(f({arg}).inner).inner"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        let stmts = statements(&mut checker, &source);
        let cells = &checker.record_cells[&id(&checker, "row")][&vec![0]];
        assert_eq!(cells.complete, complete);
        assert_eq!(
            cells.places,
            if complete {
                BTreeSet::from([(id(&checker, "a"), vec![])])
            } else {
                BTreeSet::new()
            }
        );
        let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
            panic!()
        };
        let calls = checker.calls;
        checker.record_source_cells(value, &[0]).unwrap();
        assert_eq!(checker.calls, calls);
        let error = checker
            .record_source_cells_at(value, &[0], crate::check::dependencies::calls::MAX_DEPTH)
            .err()
            .unwrap();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("call depth"));
    }
}
