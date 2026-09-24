use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn carrier_fields_survive_shared_views_copies_and_deeper_cells() {
    for tail in [
        "view:&row;cell:view.c;out:*cell",
        "view:f(&row);copy:*view;cell:copy.c;out:*cell",
        "cell:(f(f(&row))).c;out:*cell",
        "outer:{->inner:row};view:f(outer.&inner);cell:view.c;out:*cell",
    ] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}> ;f<&R>:(p<&R>){{->p}};x:=false;a:&x;row<R>:{{->c:&a}};{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "cell")];
        assert!(cells.complete, "{tail}");
        assert_eq!(cells.places, BTreeSet::from([(id(&checker, "a"), vec![])]));
        let out = id(&checker, "out");
        let x = id(&checker, "x");
        assert!(checker.pointees[&out].complete);
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn carrier_fields_keep_all_candidates_and_unknown_contents() {
    for (arg, owner, complete) in [
        ("&b", "&right", true),
        ("{->&b}", "&right", false),
        ("&b", "{->&right}", false),
    ] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}> ;f<&R>:(p<&R>,q<&R>){{->p}};x:=false;y:=true;a:&x;b:&y;left<R>:{{->c:&a}};right<R>:{{->c:{arg}}};view:f(&left,{owner});cell:view.c;out:*cell"
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
        let origins = &checker.pointees[&id(&checker, "out")];
        assert_eq!(origins.complete, complete);
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn carrier_fields_keep_nullable_nested_and_deeper_contents() {
    for (init, empty) in [("null", true), ("{->inner:{->c:&b}}", false)] {
        let source = format!(
            "<R>:<{{c<& & &boolean>}}>;<N>:<{{inner<R>}}>;<M>:<N><null>;f<&M>:(p<&M>){{->p}};x:=false;a:&x;b:&a;row<M>:{init};view:f(&row);copy:*view;|copy<N>|{{cell:copy.inner.c;out:**cell}}"
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
pub(crate) fn carrier_fields_keep_hidden_union_candidates_and_record_views() {
    let source = "<R>:<{c<& &boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{hidden<U>}>;f<&R>:(p<&W>,q<&R>){->q};x:=false;y:=true;a:&x;b:&y;one<R>:{->c:&a};two<R>:{->c:&b};pack<W>:{->hidden:{->view:&two}};view:f(&pack,&one);cell:view.c;out:*cell";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "out")];
    assert!(origins.complete);
    assert_eq!(
        origins.roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
}

#[test]
pub(crate) fn carrier_field_queries_keep_budgets_and_do_not_replay() {
    let source = "<R>:<{c<& &boolean>}>;f<&R>:(p<&R>){->p};x:=false;a:&x;row<R>:{->c:&a};cell:(f(f(&row))).c";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_cell(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    assert_eq!(
        checker
            .reference_cell_at(value, crate::check::dependencies::calls::MAX_DEPTH)
            .err()
            .unwrap()
            .code,
        "B001"
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(checker.reference_cell(value).err().unwrap().code, "B001");
}

#[test]
pub(crate) fn carrier_field_reads_preserve_owner_loans_and_expiry() {
    let prefix = "<R>:<{c<& &boolean>}>;f<&R>:(p<&R>){->p};x:=false;a:&x";
    let source = format!("{prefix};row<R>:={{->c:&a}};view:f(&row);row={{->c:&a}};out:*(view.c)");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};view:f(&({{->c:&a}}));out:*(view.c)");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn carrier_fields_retain_stored_record_and_union_views() {
    for (target, tail) in [("N", "out:copy.r"), ("U", "|copy<N>|out:copy.r")] {
        let source = format!(
            "<N>:<{{r<&boolean>}}>;<B>:<{{other<boolean>}}>;<U>:<N><B>;<R>:<{{c<&{target}>}}> ;f<&R>:(p<&R>){{->p}};x:=false;data<{target}>:{{->r:&x}};row<R>:{{->c:&data}};view:f(&row);stored:view.c;copy:*stored;{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        assert!(checker.reference_cells[&id(&checker, "stored")].complete);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert!(origins.complete);
        assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
    }
}
