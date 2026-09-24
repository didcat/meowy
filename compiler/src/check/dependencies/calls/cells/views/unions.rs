use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn owned_union_projections_retain_concrete_prefixes_and_nested_calls() {
    for tail in [
        "view:f(&pack)",
        "view:g(f(&pack))",
        "outer:{->inner:pack};view:f(outer.&inner)",
        "link:&pack;view:h(&link)",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<R>:<{{item<U>}}>;f<&U>:(p<&R>){{->p.&item}};g<&U>:(p<&U>){{->p}};h<&U>:(p<& &R>){{inner:*p;->inner.&item}};x:=false;wide<U>:{{->r:&x}};pack<R>:{{->item:wide}};{tail};copy:*view;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "view")];
        let place = if tail.contains("outer") {
            (id(&checker, "outer"), vec![0, 0])
        } else {
            (id(&checker, "pack"), vec![0])
        };
        assert!(cells.complete, "{tail}");
        assert_eq!(cells.places, BTreeSet::from([place]));
        let out = checker.locals.len() - 1;
        let x = id(&checker, "x");
        assert!(checker.pointees[&out].complete);
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn owned_union_projections_merge_stored_candidates_and_unknowns() {
    for (arg, complete) in [("&other", true), ("{->&other}", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<R>:<{{item<U>;view<&U>}}>;f<&U>:(p<&R>){{->p.&item}};x:=false;y:=true;wide<U>:{{->r:&x}};other<U>:{{->r:&y}};pack<R>:{{->item:wide;->view:{arg}}};view:f(&pack);copy:*view;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "view")];
        assert_eq!(cells.complete, complete);
        let mut places = BTreeSet::from([(id(&checker, "pack"), vec![0])]);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if complete {
            places.insert((id(&checker, "other"), vec![]));
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(cells.places, places);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete);
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn owned_union_projections_keep_null_contents_and_unknown_owners() {
    for (arg, complete) in [("&pack", true), ("{->&pack}", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B><null>;<R>:<{{item<U>}}>;f<&U>:(p<&R>){{->p.&item}};pack<R>:{{->item:null}};view:f({arg});copy:*view;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        assert_eq!(
            checker.reference_cells[&id(&checker, "view")].complete,
            complete
        );
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete);
        assert!(origins.roots.is_empty());
    }
}

#[test]
pub(crate) fn owned_union_projections_do_not_skip_hidden_nested_candidates() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;<C>:<{view<&U>}>;<D>:<{other<boolean>}>;<V>:<C><D>;<R>:<{item<U>;hidden<V>}>;f<&U>:(p<&R>){copy:p.hidden;|copy<C>|->copy.view;|copy<D>|->p.&item};x:=false;y:=true;wide<U>:{->r:&x};other<U>:{->r:&y};pack<R>:{->item:wide;->hidden:{->view:&other}};view:f(&pack)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.reference_cells[&id(&checker, "view")].complete);
}

#[test]
pub(crate) fn owned_union_projection_queries_keep_budgets_and_do_not_replay() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;<R>:<{item<U>}>;f<&U>:(p<&R>){->p.&item};x:=false;wide<U>:{->r:&x};pack<R>:{->item:wide};view:f(&pack)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_cell(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(checker.reference_cell(value).err().unwrap().code, "B001");
}

#[test]
pub(crate) fn owned_union_projections_preserve_owner_loans_and_expiry() {
    let prefix = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;<R>:<{item<U>}>;f<&U>:(p<&R>){->p.&item};x:=false;y:=true;wide<U>:{->r:&x}";
    let source =
        format!("{prefix};pack<R>:={{->item:wide}};view:f(&pack);pack={{->item:wide}};copy:*view");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};view:f(&({{->item:wide}}));copy:*view");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
