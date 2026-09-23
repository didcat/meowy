use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn record_stored_result_views_keep_inline_copied_and_composed_locations() {
    for tail in [
        "r:f({->v:&row})",
        "w<W>:{->v:&row};copy:w;r:f(copy)",
        "w<W>:{->v:&row};r:f({->w})",
    ] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;<W>:<{{v<&R>}}>;f<&R>:(p<W>){{->p.v}};x:=false;row<R>:{{->c:&x}};{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        assert!(checker.reference_cells[&r].complete);
        assert_eq!(
            checker.reference_cells[&r].places,
            BTreeSet::from([(id(&checker, "row"), vec![])])
        );
        checker.mark_derived(id(&checker, "x"));
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn nested_and_nullable_containers_keep_returned_record_candidates() {
    for (value, names) in [("{->v:&one}", vec!["one", "two"]), ("null", vec!["two"])] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;<W>:<{{v<&R>}}>;<N>:<{{inner<W><null>}}>;f<&R>:(p<N>,q<&R>){{|p.inner<W>|->p.inner.v;|p.inner<null>|->q}};x:=false;y:=true;one<R>:{{->c:&x}};two<R>:{{->c:&y}};w<N>:{{->inner<W><null>:{value}}};r:f(w,&two)"
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
pub(crate) fn record_stored_return_candidates_preserve_unknown_alternatives() {
    for (value, complete, names) in [
        ("&two", true, vec!["one", "two"]),
        ("{->&two}", false, vec!["one"]),
    ] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;<W>:<{{a<&R>;b<&R>}}>;f<&R>:(p<W>){{->p.a}};x:=false;y:=true;one<R>:{{->c:&x}};two<R>:{{->c:&y}};r:f({{->a:&one;->b:{value}}})"
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
pub(crate) fn returned_record_input_field_capacity_remains_bounded() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{c<&boolean>}>;f<&R>:(p<&R>){->p};x:=false;row<R>:{->c:&x};r:f(&row)",
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
                name: format!("v{index}"),
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
