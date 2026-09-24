use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn concrete_view_fields_retain_direct_nested_and_returned_origins() {
    for tail in [
        "view:&row;out:view.r",
        "view:f(&row);copy:*view;out:copy.r",
        "out:(f(f(&row))).r",
        "outer:{->inner:row};view:f(outer.&inner);out:view.r",
    ] {
        let source = format!(
            "<R>:<{{r<&boolean>}}> ;f<&R>:(p<&R>){{->p}};x:=false;row<R>:{{->r:&x}};{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = id(&checker, "out");
        let x = id(&checker, "x");
        assert!(checker.pointees[&out].complete, "{tail}");
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn concrete_view_fields_keep_all_candidates_and_unknowns() {
    for (arg, complete) in [("&right", true), ("{->&right}", false)] {
        let source = format!(
            "<R>:<{{r<&boolean>}}> ;f<&R>:(p<&R>,q<&R>){{->p}};x:=false;y:=true;left<R>:{{->r:&x}};right<R>:{{->r:&y}};view:f(&left,{arg});out:view.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "out")];
        assert_eq!(origins.complete, complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if complete {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn concrete_view_fields_keep_nullable_and_nested_record_paths() {
    for (init, empty) in [("null", true), ("{->inner:{->r:&x}}", false)] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<N>:<{{inner<R>}}>;<M>:<N><null>;f<&M>:(p<&M>){{->p}};x:=false;row<M>:{init};view:f(&row);copy:*view;|copy<N>|out:copy.inner.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            if empty {
                BTreeSet::new()
            } else {
                BTreeSet::from([id(&checker, "x")])
            }
        );
    }
}

#[test]
pub(crate) fn concrete_view_queries_keep_depth_budgets_and_do_not_replay() {
    let source =
        "<R>:<{r<&boolean>}>;f<&R>:(p<&R>){->p};x:=false;row<R>:{->r:&x};out:(f(f(&row))).r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_origins(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    assert_eq!(
        checker
            .reference_origins_at(value, crate::check::dependencies::calls::MAX_DEPTH)
            .err()
            .unwrap()
            .code,
        "B001"
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(checker.reference_origins(value).err().unwrap().code, "B001");
}

#[test]
pub(crate) fn concrete_view_fields_preserve_lifetimes() {
    let prefix = "<R>:<{r<&boolean>}>;f<&R>:(p<&R>){->p};x:=false;y:=true";
    let source = format!("{prefix};row<R>:={{->r:&x}};view:f(&row);row={{->r:&y}};out:view.r");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};view:f(&({{->r:&x}}));out:view.r");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn concrete_view_reads_keep_required_inputs_proof_independent() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "<R>:<{r<&boolean>}>;f<&R>:(p<&R>){->p};x:=false;row<R>:{->r:&x};view:f(&row)",
    );
    checker.mark_derived(id(&checker, "x"));
    statements(&mut checker, "out:view.r;flag:*out");
    assert!(checker.derived_local(id(&checker, "flag")));
    let block = crate::parser::parse("|flag|{n:3;<T>:{-><uint8[n]>}}").unwrap();
    assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E225");
}

#[test]
pub(crate) fn concrete_view_reads_validate_owner_prefixes_and_path_bounds() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{r<&boolean>}>;x:=false;row<R>:{->r:&x};view:&row;out:view.r",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let view = id(&checker, "view");
    let x = id(&checker, "x");
    let row = id(&checker, "row");
    checker.reference_cells.get_mut(&view).unwrap().places = BTreeSet::from([(x, vec![])]);
    let origins = checker.reference_origins(value).unwrap();
    assert!(!origins.complete);
    assert!(origins.roots.is_empty());
    checker.reference_cells.get_mut(&view).unwrap().places = BTreeSet::from([(row, vec![0; 33])]);
    assert_eq!(checker.reference_origins(value).err().unwrap().code, "B001");
}
