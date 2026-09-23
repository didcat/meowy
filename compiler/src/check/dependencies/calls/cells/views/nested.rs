use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::{Field, Type};
use std::collections::BTreeSet;

#[test]
pub(crate) fn nested_borrowed_views_retain_projected_and_stored_return_cells() {
    let source = "<N>:<{a<&boolean>;c<& &boolean>}>;<R>:<{inner<&N>}>;f<& &boolean>:(p<&R>){v:p.inner;->v.&a};x:=false;y:=true;b:&y;a<N>:{->a:&x;->c:&b};row<R>:{->inner:&a};r:f(&row);out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "a"), vec![0]), (id(&checker, "b"), vec![])])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
    checker.mark_derived(id(&checker, "y"));
    assert!(checker.derived_local(id(&checker, "r")));
}

#[test]
pub(crate) fn nested_view_chains_and_subrecords_keep_exact_return_paths() {
    for source in [
        "<N>:<{c<&boolean>}>;<R>:<{inner<& &N>}>;f<& &boolean>:(p<&R>){v:*(p.inner);->v.&c};x:=false;a<N>:{->c:&x};v:&a;row<R>:{->inner:&v};r:f(&row);out:*r",
        "<N>:<{c<&boolean>}>;<R>:<{inner<&N>}>;<W>:<{outer<&R>}>;f<& &boolean>:(p<&W>){v:p.outer.inner;->v.&c};x:=false;a<N>:{->c:&x};row<R>:{->inner:&a};w<W>:{->outer:&row};r:f(&w);out:*r",
        "<N>:<{c<&boolean>}>;<R>:<{inner<&N>}>;f<& &boolean>:(p<&R>){v:p.inner;->v.&c};x:=false;y:=true;a:{->left<N>:{->c:&x};->right<N>:{->c:&y}};row<R>:{->inner:a.&left};r:f(&row);out:*r",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        assert!(checker.reference_cells[&id(&checker, "r")].complete);
        assert_eq!(
            checker.pointees[&id(&checker, "out")].roots,
            BTreeSet::from([id(&checker, "x")]),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn retargeted_nested_views_keep_unknown_alternatives_incomplete() {
    for (value, complete, names) in [("&b", true, vec!["a", "b"]), ("{->&b}", false, vec!["a"])] {
        let source = format!(
            "<N>:<{{c<&boolean>}}>;<R>:<{{inner<&N>:=}}>;f<& &boolean>:(p<&R>){{v:p.inner;->v.&c}};x:=false;y:=true;a<N>:{{->c:&x}};b<N>:{{->c:&y}};row<R>:{{->inner:=&a}};row.inner={value};r:f(&row)"
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
                .map(|name| (id(&checker, name), vec![0]))
                .collect()
        );
    }
}

#[test]
pub(crate) fn nested_nullable_views_keep_absent_stored_cells_empty() {
    for (value, names) in [("{->c:&a}", vec!["a", "b"]), ("null", vec!["b"])] {
        let source = format!(
            "<N>:<{{c<& &boolean>}}>;<M>:<N><null>;<R>:<{{inner<&M>}}>;f<& &boolean>:(p<&R>,q<& &boolean>){{v:*(p.inner);|v<N>|->v.c;|v<null>|->q}};x:=false;y:=true;a:&x;b:&y;inner<M>:{value};row<R>:{{->inner:&inner}};r:f(&row,&b)"
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
pub(crate) fn unknown_nested_returned_cell_views_keep_total_depth_bounded() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<N>:<{c<&boolean>}>;f<& &boolean>:(p<&N>){->p.&c};x:=false;a<N>:{->c:&x};r:f(&a)",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let crate::hir::ExprKind::Call { args, .. } = &value.kind else {
        panic!()
    };
    let mut ty = args[0].ty.clone();
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
    let error = checker
        .returned_record_cells(crate::check::dependencies::Cells::default(), &ty, value, 2)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("returned record cell"));
}
