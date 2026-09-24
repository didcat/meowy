use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::{Field, Type};
use std::collections::BTreeSet;

#[test]
pub(crate) fn nested_borrowed_views_retain_external_and_owned_candidates() {
    for body in ["->p.inner.r", "view:p.inner;->view.&flag"] {
        let source = format!(
            "<N>:<{{r<&boolean>;flag<boolean>}}>;<R>:<{{inner<&N>}}>;f<&boolean>:(p<&R>){{{body}}};x:=false;a<N>:{{->r:&x;->flag:true}};row<R>:{{->inner:&a}};r:f(&row)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        let x = id(&checker, "x");
        assert!(checker.pointees[&r].complete);
        assert_eq!(
            checker.pointees[&r].roots,
            BTreeSet::from([x, id(&checker, "a")])
        );
        checker.mark_derived(x);
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn nested_view_chains_and_subrecords_keep_selected_locations() {
    for source in [
        "<N>:<{r<&boolean>}>;<R>:<{inner<& &N>}>;f<&boolean>:(p<&R>){->(*(p.inner)).r};x:=false;a<N>:{->r:&x};v:&a;row<R>:{->inner:&v};r:f(&row)",
        "<N>:<{r<&boolean>}>;<R>:<{inner<&N>}>;<W>:<{outer<&R>}>;f<&boolean>:(p<&W>){->p.outer.inner.r};x:=false;a<N>:{->r:&x};row<R>:{->inner:&a};outer<W>:{->outer:&row};r:f(&outer)",
        "<N>:<{r<&boolean>}>;<R>:<{inner<&N>}>;f<&boolean>:(p<&R>){->p.inner.r};x:=false;y:=true;a:{->left<N>:{->r:&x};->right<N>:{->r:&y}};row<R>:{->inner:a.&left};r:f(&row)",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert!(origins.complete, "{source}");
        assert_eq!(
            origins.roots,
            BTreeSet::from([id(&checker, "x")]),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn retargeted_nested_views_preserve_known_and_unknown_alternatives() {
    for (value, complete, names) in [("&b", true, vec!["x", "y"]), ("{->&b}", false, vec!["x"])] {
        let source = format!(
            "<N>:<{{r<&boolean>}}>;<R>:<{{inner<&N>:=}}>;f<&boolean>:(p<&R>){{->p.inner.r}};x:=false;y:=true;a<N>:{{->r:&x}};b<N>:{{->r:&y}};row<R>:{{->inner:=&a}};row.inner={value};r:f(&row)"
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
pub(crate) fn unknown_nested_types_still_obey_total_record_depth() {
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, "x:=false;row:{->r:&x};view:&row");
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let mut ty = value.ty.clone();
    for _ in 0..=super::MAX_DEPTH {
        ty = Type::Reference(Box::new(Type::Record {
            primary: Box::new(Type::Null),
            fields: vec![Field {
                name: "inner".into(),
                ty,
                mutable: false,
            }],
        }));
    }
    let result = Type::Reference(Box::new(Type::Bool));
    let error = checker
        .call_record_view_origins(value, &[], &ty, &result, 0)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("record call"));
}

#[test]
pub(crate) fn stored_record_origin_traversal_keeps_initial_depth() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{r<&boolean>}>;x:=false;row<R>:{->r:&x};view:&row",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let result = Type::Reference(Box::new(Type::Bool));
    let cells = crate::check::dependencies::Cells::default();
    assert!(
        checker
            .call_record_cell_origins(cells.clone(), value, &value.ty, &result, 0, 31)
            .is_ok()
    );
    let error = checker
        .call_record_cell_origins(cells, value, &value.ty, &result, 0, 32)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("record call"));
}
