use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn returned_union_carriers_retain_direct_nested_and_stored_candidates() {
    for tail in [
        "cell:f(&view);copy:**cell",
        "cell:f(f(&view));old:cell;copy:**old",
        "copy:**(f(&view))",
        "pack:{->cell:&view};cell:g(pack);copy:**cell",
        "inner:&view;cell:h(&inner);copy:***cell",
        "pack:{->view:view};cell:k(&pack);copy:**cell",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<R>:<{{cell<& &U>}}>;<V>:<{{view<&U>}}>;f<& &U>:(p<& &U>){{->p}};g<& &U>:(p<R>){{->p.cell}};h<& & &U>:(p<& & &U>){{->p}};k<& &U>:(p<&V>){{->p.&view}};x:=false;wide<U>:{{->r:&x}};view:&wide;{tail};|copy<A>|out:copy.r"
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
pub(crate) fn returned_union_carriers_match_deeper_inputs_and_preserve_unknowns() {
    for (arg, complete) in [("&b", true), ("{->&b}", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;f<& &U>:(p<& & &U>,q<& &U>){{->q}};x:=false;y:=true;left<U>:{{->r:&x}};right<U>:{{->r:&y}};a:&left;b:&right;outer:&a;cell:f(&outer,{arg});copy:**cell;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "cell")];
        assert_eq!(cells.complete, complete);
        let mut places = BTreeSet::from([(id(&checker, "a"), vec![])]);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if complete {
            places.insert((id(&checker, "b"), vec![]));
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(cells.places, places);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete);
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn returned_union_carriers_keep_nullable_payloads_and_inner_carriers() {
    for (init, empty) in [("null", true), ("{->r:&a}", false)] {
        let source = format!(
            "<A>:<{{r<& &boolean>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B><null>;f<& &U>:(p<& &U>){{->p}};x:=false;a:&x;wide<U>:{init};view:&wide;cell:f(&view);copy:**cell;|copy<A>|out:*(copy.r)"
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
pub(crate) fn returned_union_carrier_queries_keep_call_depth_and_do_not_replay() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<& &U>:(p<& &U>){->p};x:=false;wide<U>:{->r:&x};view:&wide;cell:f(f(&view))";
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
pub(crate) fn returned_union_terminal_classification_keeps_modes_and_bounds() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x};view:&wide",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let union = checker.locals[id(&checker, "wide")].clone();
    for count in [1, 2, MAX_CELL_DEPTH + 1] {
        let mut ty = union.clone();
        for _ in 0..count {
            ty = Type::Reference(Box::new(ty));
        }
        assert_eq!(checker.shared_cell_depth(&ty, value).unwrap(), Some(count));
    }
    let mut ty = union.clone();
    for _ in 0..MAX_CELL_DEPTH + 2 {
        ty = Type::Reference(Box::new(ty));
    }
    assert_eq!(
        checker.shared_cell_depth(&ty, value).unwrap_err().code,
        "B001"
    );
    let exclusive = Type::Reference(Box::new(Type::Exclusive(Box::new(union.clone()))));
    assert_eq!(checker.shared_cell_depth(&exclusive, value).unwrap(), None);
    let Type::Union(members) = union else {
        panic!()
    };
    let large = Type::Reference(Box::new(Type::Union(vec![
        members[0].clone();
        MAX_ROOTS + 1
    ])));
    assert!(checker.shared_cell_depth(&large, value).is_err());
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.shared_cell_depth(&value.ty, value).is_err());
}

#[test]
pub(crate) fn returned_union_carriers_preserve_live_loans_and_temporary_expiry() {
    let prefix = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<& &U>:(p<& &U>){->p};x:=false;y:=true;left<U>:{->r:&x};right<U>:{->r:&y}";
    let source = format!("{prefix};view:=&left;cell:f(&view);view=&right;copy:**cell");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};cell:f(&(&left));copy:**cell");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn unmatched_union_inputs_do_not_hide_untraversed_return_candidates() {
    for source in [
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;<C>:<{cell<& &U>}>;<D>:<{other<boolean>}>;<V>:<C><D>;f<& &U>:(p<&V>,q<& &U>){copy:*p;|copy<C>|->copy.cell;|copy<D>|->q};x:=false;y:=true;wide<U>:{->r:&x};other<U>:{->r:&y};view:&wide;hidden:&other;holder<V>:{->cell:&hidden};out:f(&holder,&view)",
        "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{plain<&R>;view<&U>}>;f<&R>:(p<&W>){copy:*(p.view);|copy<A>|->copy.view;|copy<B>|->p.plain};x:=false;y:=true;data<R>:{->r:&x};other<R>:{->r:&y};wide<U>:{->view:&other};holder<W>:{->plain:&data;->view:&wide};out:f(&holder)",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        assert!(!checker.reference_cells[&id(&checker, "out")].complete);
    }
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;<V>:<boolean><int32>;f<& &U>:(p<&V>,q<& &U>){->q};x:=false;wide<U>:{->r:&x};view:&wide;plain<V>:false;out:f(&plain,&view)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "out")];
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "view"), vec![])])
    );
}
