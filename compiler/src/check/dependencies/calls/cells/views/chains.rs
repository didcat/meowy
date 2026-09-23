use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Type;
use std::collections::BTreeSet;

#[test]
pub(crate) fn shared_record_chains_retain_projected_return_cell_locations() {
    for (param, body, tail) in [
        ("& &R", "v:*p;->v.&c", "v:&row;r:f(&v)"),
        ("& & &R", "v:**p;->v.&c", "v:&row;w:&v;r:f(&w)"),
    ] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;f<& &boolean>:(p<{param}>){{{body}}};x:=false;row<R>:{{->c:&x}};{tail};out:*r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "r")];
        assert!(cells.complete);
        assert_eq!(
            cells.places,
            BTreeSet::from([(id(&checker, "row"), vec![0])])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "out")].roots,
            BTreeSet::from([id(&checker, "x")])
        );
    }
    let source = "<R>:<{c<&boolean>}>;f<& &boolean>:(p<& &R>){v:*p;->v.&c};x:=false;row<R>:{->c:&x};value:**(f(&(&row)))";
    crate::compile(source).unwrap();
    let source = "<R>:<{c<&boolean>}>;f<& &boolean>:(p<& &R>){v:*p;->v.&c};x:=false;row<R>:{->c:&x};r:f(&(&row));out:*r";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn record_stored_view_chains_keep_mixed_return_candidates() {
    let source = "<R>:<{a<&boolean>;c<& &boolean>}>;<W>:<{v<& &R>}>;f<& &boolean>:(p<W>){v:*(p.v);->v.&a};x:=false;y:=true;b:&y;row<R>:{->a:&x;->c:&b};v:&row;r:f({->v:&v});out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "row"), vec![0]), (id(&checker, "b"), vec![])])
    );
    checker.mark_derived(id(&checker, "y"));
    assert!(checker.derived_local(id(&checker, "out")));
}

#[test]
pub(crate) fn unknown_record_chain_cells_keep_known_candidates_incomplete() {
    let source = "<R>:<{c<&boolean>}>;f<& &boolean>:(p<& & &R>,q<& &boolean>){->q};x:=false;y:=true;row<R>:{->c:&x};v:&row;w:{->&v};b:&y;r:f(&w,&b);out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(!cells.complete);
    assert_eq!(cells.places, BTreeSet::from([(id(&checker, "b"), vec![])]));
    assert!(!checker.pointees[&id(&checker, "out")].complete);
}

#[test]
pub(crate) fn nullable_record_input_chains_keep_null_carriers_empty() {
    for (init, names) in [("{->c:&a}", vec!["a", "b"]), ("null", vec!["b"])] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}>;<M>:<R><null>;f<& &boolean>:(p<& &M>,q<& &boolean>){{v:**p;|v<R>|->v.c;|v<null>|->q}};x:=false;y:=true;a:&x;b:&y;row<M>:{init};v:&row;r:f(&v,&b)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "r")];
        assert!(cells.complete);
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
pub(crate) fn returned_cell_record_inputs_enforce_shared_type_depth() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{c<&boolean>}>;f<& &boolean>:(p<&R>){->p.&c};x:=false;row<R>:{->c:&x};r:f(&row)",
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
