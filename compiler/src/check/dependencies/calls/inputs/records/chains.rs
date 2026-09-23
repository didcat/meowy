use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn chains_to_borrowed_records_keep_stored_reference_owners() {
    for source in [
        "<R>:<{r<&boolean>}>;f<&boolean>:(p<& &R>){->(*p).r};x:=false;row<R>:{->r:&x};a:&row;r:f(&a)",
        "<R>:<{r<&boolean>}>;f<&boolean>:(p<& & &R>){->(**p).r};x:=false;row<R>:{->r:&x};a:&row;b:&a;r:f(&b)",
        "<R>:<{r<&boolean>}>;f<&boolean>:(p<& &R>){->(*p).r};x:=false;row<R>:{->r:&x};r:f(&(&row))",
        "<R>:<{r<&boolean>}>;<W>:<{c<& &R>}>;f<&boolean>:(p<W>){->(*(p.c)).r};x:=false;row<R>:{->r:&x};a:&row;r:f({->c:&a})",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let r = id(&checker, "r");
        let x = id(&checker, "x");
        assert!(checker.pointees[&r].complete, "{source}");
        assert_eq!(checker.pointees[&r].roots, BTreeSet::from([x]), "{source}");
        checker.mark_derived(x);
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn record_chains_keep_owned_and_carried_reference_candidates() {
    let source = "<R>:<{c<& &boolean>;flag<boolean>}>;f<&boolean>:(p<& &R>){view:*p;->view.&flag};x:=false;a:&x;row<R>:{->c:&a;->flag:true};view:&row;r:f(&view)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(origins.complete);
    assert_eq!(
        origins.roots,
        BTreeSet::from([id(&checker, "row"), id(&checker, "x")])
    );
}

#[test]
pub(crate) fn retargeted_record_chain_cells_preserve_unknown_alternatives() {
    for (value, complete, names) in [("&b", true, vec!["x", "y"]), ("{->&b}", false, vec!["x"])] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;f<&boolean>:(p<& & &R>){{->(**p).r}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};a:&one;b:&two;c:=&a;c={value};r:f(&c)"
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
pub(crate) fn record_chain_type_traversal_keeps_depth_and_work_bounds() {
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, "x:=false;row:{->r:&x};view:&row");
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let mut ty = value.ty.clone();
    let limit = crate::check::dependencies::references::MAX_CELL_DEPTH;
    for _ in 0..limit {
        ty = crate::hir::Type::Reference(Box::new(ty));
    }
    assert_eq!(
        checker.call_shared_view(&ty, value).unwrap().unwrap().1,
        limit
    );
    ty = crate::hir::Type::Reference(Box::new(ty));
    let error = checker.call_shared_view(&ty, value).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("cell depth"));
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(
        checker
            .call_shared_view(&value.ty, value)
            .err()
            .unwrap()
            .code,
        "B001"
    );
}
