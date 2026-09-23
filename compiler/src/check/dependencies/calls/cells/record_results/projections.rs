use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::{Field, Type};
use std::collections::BTreeSet;

#[test]
pub(crate) fn projected_record_results_keep_nested_owned_locations() {
    for tail in [
        "r:f(&row)",
        "v:&row;r:f(v)",
        "outer:{->row:row};r:f(outer.&row)",
    ] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;<N>:<{{inner<R>}}>;<W>:<{{nested<N>}}>;f<&R>:(p<&W>){{->p.nested.&inner}};x:=false;row<W>:{{->nested:{{->inner<R>:{{->c:&x}}}}}};{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        assert!(checker.reference_cells[&r].complete);
        let location = if tail.contains("outer") {
            (id(&checker, "outer"), vec![0, 0, 0])
        } else {
            (id(&checker, "row"), vec![0, 0])
        };
        assert_eq!(
            checker.reference_cells[&r].places,
            BTreeSet::from([location])
        );
        checker.mark_derived(id(&checker, "x"));
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn projected_record_results_keep_owned_and_stored_chain_candidates() {
    let source = "<R>:<{c<&boolean>}>;<W>:<{owned<R>;view<& &R>}>;f<&R>:(p<&W>){->p.&owned};read<&boolean>:(p<&R>){->p.c};x:=false;y:=true;a<R>:{->c:&y};v:&a;row<W>:{->owned<R>:{->c:&x};->view:&v};r:f(&row);out:read(r)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "row"), vec![0]), (id(&checker, "a"), vec![])])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
}

#[test]
pub(crate) fn unknown_stored_views_keep_known_projected_locations_incomplete() {
    let source = "<R>:<{c<&boolean>}>;<W>:<{owned<R>;view<&R>}>;f<&R>:(p<&W>){->p.&owned};x:=false;y:=true;a<R>:{->c:&y};row<W>:{->owned<R>:{->c:&x};->view:{->&a}};r:f(&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(!cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "row"), vec![0])])
    );
}

#[test]
pub(crate) fn nullable_borrowed_inputs_keep_absent_record_view_candidates_empty() {
    for (value, names) in [("{->view:&a}", vec!["a", "b"]), ("null", vec!["b"])] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;<W>:<{{view<&R>}}>;<M>:<W><null>;f<&R>:(p<&M>,q<&R>){{v:*p;|v<W>|->v.view;|v<null>|->q}};x:=false;y:=true;a<R>:{{->c:&x}};b<R>:{{->c:&y}};row<M>:{value};r:f(&row,&b)"
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
pub(crate) fn returned_record_projection_traversal_keeps_unknown_depth_bounded() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{c<&boolean>}>;f<&R>:(p<&R>){->p};x:=false;row<R>:{->c:&x};r:f(&row)",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let mut ty = value.ty.clone();
    for _ in 0..=super::MAX_DEPTH {
        ty = Type::Reference(Box::new(Type::Record {
            primary: Box::new(Type::Null),
            fields: vec![Field {
                name: "view".into(),
                ty,
                mutable: false,
            }],
        }));
    }
    let error = checker
        .returned_record_cells(
            crate::check::dependencies::Cells::default(),
            &ty,
            value,
            None,
        )
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("returned record cell"));
}
