use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn borrowed_record_calls_keep_stored_reference_owners() {
    for tail in [
        "r:f(&a)",
        "p:&a;copy:p;r:f(copy)",
        "row:{->view:&a};r:f(row.view)",
        "outer:{->inner:a};r:f(outer.&inner)",
        "r:f(&{->r:&x})",
    ] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;f<&boolean>:(p<&R>){{->p.r}};x:=false;a<R>:{{->r:&x}};{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        let x = id(&checker, "x");
        assert!(checker.pointees[&r].complete, "{tail}");
        assert_eq!(checker.pointees[&r].roots, BTreeSet::from([x]), "{tail}");
        checker.mark_derived(x);
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn borrowed_records_combine_owned_projections_and_nested_view_origins() {
    for body in ["->p.inner.r", "->p.&flag"] {
        let source = format!(
            "<N>:<{{r<&boolean>}}>;<R>:<{{inner<N>;flag<boolean>}}>;f<&boolean>:(p<&R>){{{body}}};x:=false;a<R>:{{->inner:{{->r:&x}};->flag:true}};r:f(&a)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            BTreeSet::from([id(&checker, "a"), id(&checker, "x")])
        );
    }
}

#[test]
pub(crate) fn borrowed_record_retargets_and_unknown_sources_remain_conservative() {
    for (value, names, complete) in [("&b", vec!["x", "y"], true), ("{->&b}", vec!["x"], false)] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;f<&boolean>:(p<&R>){{->p.r}};x:=false;y:=true;a<R>:{{->r:&x}};b<R>:{{->r:&y}};p:=&a;p={value};r:f(p)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert_eq!(origins.complete, complete);
        assert_eq!(
            origins.roots,
            names.iter().map(|name| id(&checker, name)).collect()
        );
    }
    let source = "<R>:<{r<&boolean>;flag<boolean>}>;f<&boolean>:(p<&R>){->p.&flag};x:=false;a<R>:{->r:{->&x};->flag:true};r:f(&a)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(!origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "a")]));
}

#[test]
pub(crate) fn borrowed_record_calls_keep_nested_record_view_gates_and_lifetime_errors() {
    let source = "<N>:<{r<&boolean>}>;<R>:<{inner<&N>}>;f<&boolean>:(p<&R>){->p.inner.r};x:=false;a<N>:{->r:&x};row<R>:{->inner:&a};r:f(&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.pointees[&id(&checker, "r")].complete);
    let source = "<R>:<{r<&boolean>}>;f<&boolean>:(p<&R>){local:false;->&local};x:=false;a<R>:{->r:&x};r:f(&a)";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn record_arguments_can_supply_stored_borrowed_record_views() {
    let source = "<R>:<{r<&boolean>}>;<W>:<{view<&R>}>;f<&boolean>:(p<W>){->p.view.r};x:=false;a<R>:{->r:&x};r:f({->view:&a})";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
}
