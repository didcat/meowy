use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn borrowed_record_carrier_fields_recover_deep_and_nested_owners() {
    for source in [
        "<R>:<{c<& &boolean>}>;f<&boolean>:(p<&R>){->*(p.c)};x:=false;a:&x;row<R>:{->c:&a};r:f(&row)",
        "<R>:<{c<& & &boolean>}>;f<&boolean>:(p<&R>){->**(p.c)};x:=false;a:&x;b:&a;row<R>:{->c:&b};r:f(&row)",
        "<N>:<{c<& &boolean>}>;<R>:<{inner<N>}>;f<&boolean>:(p<&R>){->*(p.inner.c)};x:=false;a:&x;row<R>:{->inner:{->c:&a}};r:f(&row)",
        "<R>:<{c<& &boolean[2]>}>;f<&boolean>:(p<&R>){->&((*(p.c))[1])};x<boolean[2]>:=[false,true];a:&x;row<R>:{->c:&a};r:f(&row)",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let x = id(&checker, "x");
        let r = id(&checker, "r");
        assert!(checker.pointees[&r].complete, "{source}");
        assert_eq!(checker.pointees[&r].roots, BTreeSet::from([x]), "{source}");
        checker.mark_derived(x);
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn carrier_fields_keep_owned_and_external_return_candidates() {
    let source = "<R>:<{c<& &boolean>;flag<boolean>}>;f<&boolean>:(p<&R>){->p.&flag};x:=false;a:&x;row<R>:{->c:&a;->flag:true};r:f(&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(origins.complete);
    assert_eq!(
        origins.roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "row")])
    );
}

#[test]
pub(crate) fn retargeted_carrier_fields_preserve_known_and_unknown_alternatives() {
    for (value, complete, names) in [("&b", true, vec!["x", "y"]), ("{->&b}", false, vec!["x"])] {
        let source = format!(
            "<R>:<{{c<& &boolean>:=}}>;f<&boolean>:(p<&R>){{->*(p.c)}};x:=false;y:=true;a:&x;b:&y;row<R>:{{->c:=&a}};row.c={value};r:f(&row)"
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
}

#[test]
pub(crate) fn unknown_carrier_pointees_and_work_exhaustion_remain_explicit() {
    let source = "<R>:<{c<& &boolean>;flag<boolean>}>;f<&boolean>:(p<&R>){->p.&flag};x:=false;a:{->&x};row<R>:{->c:&a;->flag:true};r:f(&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(!origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "row")]));
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let cells = checker.record_cells[&id(&checker, "row")][&vec![0]].clone();
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(
        checker
            .call_stored_origins(cells, value, 1)
            .err()
            .unwrap()
            .code,
        "B001"
    );
}
