use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn returned_record_views_keep_locations_through_copies_and_nested_calls() {
    for tail in ["r:f(&row)", "r:f(f(&row))", "v:f(&row);r:v"] {
        let source =
            format!("<R>:<{{c<&boolean>}}>;f<&R>:(p<&R>){{->p}};x:=false;row<R>:{{->c:&x}};{tail}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        let cells = &checker.reference_cells[&r];
        assert!(cells.complete);
        assert_eq!(
            cells.places,
            BTreeSet::from([(id(&checker, "row"), vec![])])
        );
        checker.mark_derived(id(&checker, "x"));
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn returned_record_views_keep_every_compatible_argument_location() {
    let source = "<R>:<{c<&boolean>}>;f<&R>:(a<&R>,b<&R>,n<int32>){->a};x:=false;y:=true;a<R>:{->c:&x};b<R>:{->c:&y};r:f(&a,&b,7)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "a"), vec![]), (id(&checker, "b"), vec![])])
    );
    checker.mark_derived(id(&checker, "y"));
    assert!(checker.derived_local(id(&checker, "r")));
}

#[test]
pub(crate) fn unknown_returned_record_candidates_retain_known_incomplete_locations() {
    let source = "<R>:<{c<&boolean>}>;f<&R>:(a<&R>,b<&R>){->a};x:=false;y:=true;a<R>:{->c:&x};b<R>:{->c:&y};r:f(&a,{->&b})";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(!cells.complete);
    assert_eq!(cells.places, BTreeSet::from([(id(&checker, "a"), vec![])]));
}

#[test]
pub(crate) fn returned_record_locations_feed_later_origin_queries() {
    for (value, complete) in [("&x", true), ("{->&x}", false)] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;f<&R>:(p<&R>){{->p}};read<&boolean>:(p<&R>){{->p.c}};x:=false;row<R>:{{->c:{value}}};r:read(f(&row))"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert_eq!(origins.complete, complete);
        let expected = if complete {
            BTreeSet::from([id(&checker, "x")])
        } else {
            BTreeSet::new()
        };
        assert_eq!(origins.roots, expected);
    }
}

#[test]
pub(crate) fn returned_record_views_preserve_lifetimes_and_unsupported_projections() {
    let source = "<R>:<{c<&boolean>}>;f<&R>:(p<&R>){row<R>:{->c:p.c};->&row};x:=false;row<R>:{->c:&x};r:f(&row)";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
    let source = "<R>:<{c<&boolean>}>;<W>:<{inner<R>}>;f<&R>:(p<&W>){->p.&inner};x:=false;row<W>:{->inner<R>:{->c:&x}};r:f(&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    assert!(!checker.reference_cells[&id(&checker, "r")].complete);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let error = checker
        .reference_cell_at(value, super::super::super::MAX_DEPTH + 1)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("call depth"));
}
