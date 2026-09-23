use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn record_stored_returned_cells_survive_inline_copied_and_nested_sources() {
    for tail in [
        "r:f({->c:&a})",
        "row<R>:{->c:&a};copy:row;r:f(copy)",
        "row<R>:{->c:&a};r:f({->row})",
    ] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}>;f<& &boolean>:(p<R>){{->p.c}};x:=false;a:&x;{tail};out:*r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        let x = id(&checker, "x");
        assert!(checker.reference_cells[&r].complete);
        assert_eq!(
            checker.reference_cells[&r].places,
            BTreeSet::from([(id(&checker, "a"), vec![])])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "out")].roots,
            BTreeSet::from([x])
        );
        checker.mark_derived(x);
        assert!(checker.derived_local(r));
    }
    let source = "<R>:<{c<& & &boolean>}>;<N>:<{inner<R>}>;f<& &boolean>:(p<N>){->*(p.inner.c)};x:=false;a:&x;b:&a;r:f({->inner:{->c:&b}});out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(checker.reference_cells[&id(&checker, "r")].complete);
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([id(&checker, "x")])
    );
}

#[test]
pub(crate) fn nullable_record_return_candidates_keep_null_empty() {
    for (init, names) in [("{->c:&a}", vec!["a", "b"]), ("null", vec!["b"])] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}>;f<& &boolean>:(p<R><null>,q<& &boolean>){{|p<R>|->p.c;|p<null>|->q}};x:=false;y:=true;a:&x;b:&y;row<R><null>:{init};r:f(row,&b)"
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
pub(crate) fn matching_record_fields_keep_all_candidates_and_unknowns() {
    for (value, complete, names) in [("&b", true, vec!["a", "b"]), ("{->&b}", false, vec!["a"])] {
        let source = format!(
            "<R>:<{{a<& &boolean>;b<& &boolean>}}>;f<& &boolean>:(p<R>){{->p.a}};x:=false;y:=true;a:&x;b:&y;r:f({{->a:&a;->b:{value}}})"
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
pub(crate) fn heterogeneous_record_return_candidates_remain_incomplete() {
    let source = "<A>:<{c<& &boolean>}>;<B>:<{n<int32>}>;f<& &boolean>:(p<A><B>,q<& &boolean>){|p<A>|->p.c;|p<B>|->q};x:=false;a:&x;row<A><B>:{->c:&a};r:f(row,&a)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.reference_cells[&id(&checker, "r")].complete);
}

#[test]
pub(crate) fn returned_record_field_expansion_respects_capacity() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "f<& &boolean>:(p<& &boolean>){->p};x:=false;a:&x;r:f(&a)",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let crate::hir::ExprKind::Call { args, .. } = &value.kind else {
        panic!()
    };
    let mut arg = args[0].clone();
    arg.ty = crate::hir::Type::Record {
        primary: Box::new(crate::hir::Type::Null),
        fields: (0..=super::MAX_FIELDS)
            .map(|index| crate::hir::Field {
                name: format!("c{index}"),
                ty: args[0].ty.clone(),
                mutable: false,
            })
            .collect(),
    };
    let error = checker
        .call_reference_cells(value, &[arg], 0)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("input capacity"));
}
