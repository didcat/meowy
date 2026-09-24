use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn returned_union_views_match_direct_deeper_and_stored_inputs() {
    for tail in [
        "view:f(&wide);copy:*view",
        "view:f(f(&wide));old:view;copy:*old",
        "copy:*(f(&wide))",
        "inner:&wide;view:g(&inner);copy:*view",
        "pack:{->view:&wide};view:h(pack);copy:*view",
        "pack:{->view:&wide};view:k(&pack);copy:*view",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<R>:<{{view<&U>}}>;f<&U>:(p<&U>){{->p}};g<&U>:(p<& &U>){{->*p}};h<&U>:(p<R>){{->p.view}};k<&U>:(p<&R>){{->p.view}};x:=false;wide<U>:{{->r:&x}};{tail};|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = checker.locals.len() - 1;
        let x = id(&checker, "x");
        assert!(checker.pointees[&out].complete, "{tail}");
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn returned_union_views_merge_all_candidates_and_keep_unknowns() {
    for (arg, complete) in [("&right", true), ("{->&right}", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;f<&U>:(p<& &U>,q<&U>){{->q}};x:=false;y:=true;left<U>:{{->r:&x}};right<U>:{{->r:&y}};a:&left;view:f(&a,{arg});copy:*view;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "view")];
        assert_eq!(cells.complete, complete);
        let mut places = BTreeSet::from([(id(&checker, "left"), vec![])]);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if complete {
            places.insert((id(&checker, "right"), vec![]));
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(cells.places, places);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete);
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn returned_union_views_retain_null_and_carrier_payloads() {
    for (init, empty) in [("null", true), ("{->r:&a}", false)] {
        let source = format!(
            "<A>:<{{r<& &boolean>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B><null>;f<&U>:(p<&U>){{->p}};x:=false;a:&x;wide<U>:{init};view:f(&wide);copy:*view;|copy<A>|out:*(copy.r)"
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
pub(crate) fn returned_union_views_retain_hidden_direct_input_candidates() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;<C>:<{view<&U>}>;<D>:<{other<boolean>}>;<V>:<C><D>;f<&U>:(p<&V>,q<&U>){copy:*p;|copy<C>|->copy.view;|copy<D>|->q};x:=false;y:=true;wide<U>:{->r:&x};other<U>:{->r:&y};pack<V>:{->view:&other};view:f(&pack,&wide)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "view")];
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([
            (id(&checker, "wide"), vec![]),
            (id(&checker, "other"), vec![])
        ])
    );
}

#[test]
pub(crate) fn returned_union_views_bound_queries_without_replay() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<&U>:(p<&U>){->p};x:=false;wide<U>:{->r:&x};view:f(f(&wide))";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_cell(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    let error = checker
        .reference_cell_at(value, super::super::MAX_DEPTH)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("call depth"));
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.reference_cell(value).is_err());
}

#[test]
pub(crate) fn returned_union_views_preserve_live_loans_and_temporary_expiry() {
    let prefix =
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<&U>:(p<&U>){->p};x:=false;y:=true";
    let source = format!("{prefix};wide<U>:={{->r:&x}};view:f(&wide);wide={{->r:&y}};copy:*view");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};wide<U>:{{->r:&x}};view:f(&({{->item:wide}}.item));copy:*view");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
