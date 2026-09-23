use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Type;
use std::collections::BTreeSet;

#[test]
pub(crate) fn returned_record_views_follow_direct_deep_and_stored_input_chains() {
    for source in [
        "<R>:<{c<&boolean>}>;f<&R>:(p<& &R>){->*p};x:=false;row<R>:{->c:&x};v:&row;r:f(&v)",
        "<R>:<{c<&boolean>}>;f<&R>:(p<& & &R>){->**p};x:=false;row<R>:{->c:&x};v:&row;w:&v;r:f(&w)",
        "<R>:<{c<&boolean>}>;<W>:<{v<& &R>}>;f<&R>:(p<W>){->*(p.v)};x:=false;row<R>:{->c:&x};v:&row;r:f({->v:&v})",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
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
pub(crate) fn returned_record_chains_keep_mixed_and_unknown_candidates() {
    for (value, complete, names) in [
        ("&one", true, vec!["one", "two"]),
        ("{->&one}", false, vec!["two"]),
    ] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;f<&R>:(p<& &R>,q<&R>){{->q}};x:=false;y:=true;one<R>:{{->c:&x}};two<R>:{{->c:&y}};v:{value};r:f(&v,&two)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "r")];
        assert_eq!(cells.complete, complete);
        assert_eq!(
            cells.places,
            names
                .iter()
                .map(|name| (id(&checker, name), vec![]))
                .collect()
        );
    }
}

#[test]
pub(crate) fn temporary_record_input_chains_keep_statement_lifetimes() {
    let source = "<R>:<{c<&boolean>}>;f<&R>:(p<& &R>){->*p};read<boolean>:(p<&R>){->*(p.c)};x:=false;row<R>:{->c:&x};out:read(f(&(&row)))";
    crate::compile(source).unwrap();
    let source =
        "<R>:<{c<&boolean>}>;f<&R>:(p<& &R>){->*p};x:=false;row<R>:{->c:&x};r:f(&(&row));out:r.c";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn returned_record_view_input_chains_keep_depth_limits() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{c<&boolean>}>;f<&R>:(p<&R>){->p};x:=false;row<R>:{->c:&x};r:f(&row)",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let crate::hir::ExprKind::Call { args, .. } = &value.kind else {
        panic!()
    };
    let mut arg = args[0].clone();
    for _ in 0..=crate::check::dependencies::references::MAX_CELL_DEPTH {
        arg.ty = Type::Reference(Box::new(arg.ty));
    }
    let error = checker
        .call_reference_cells(value, &[arg], 0)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("cell depth"));
}
