use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Type;
use std::collections::BTreeSet;

#[test]
pub(crate) fn returned_record_carriers_keep_intermediate_cells_through_nested_calls() {
    for tail in ["r:f(&v)", "r:f(f(&v))", "a:f(&v);r:a"] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;f<& &R>:(p<& &R>){{->p}};x:=false;row<R>:{{->c:&x}};v:&row;{tail};out:*r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        assert!(checker.reference_cells[&r].complete);
        assert_eq!(
            checker.reference_cells[&r].places,
            BTreeSet::from([(id(&checker, "v"), vec![])])
        );
        assert_eq!(
            checker.reference_cells[&id(&checker, "out")].places,
            BTreeSet::from([(id(&checker, "row"), vec![])])
        );
        checker.mark_derived(id(&checker, "x"));
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn deeper_and_record_stored_chains_keep_matching_carriers() {
    for source in [
        "<R>:<{c<&boolean>}>;f<& &R>:(p<& & &R>){->*p};x:=false;row<R>:{->c:&x};v:&row;w:&v;r:f(&w)",
        "<R>:<{c<&boolean>}>;<W>:<{v<& & &R>}>;f<& &R>:(p<W>){->*(p.v)};x:=false;row<R>:{->c:&x};v:&row;w:&v;r:f({->v:&w})",
        "<R>:<{c<&boolean>}>;<W>:<{v<& &R>}>;f<& &R>:(p<&W>){->p.v};x:=false;row<R>:{->c:&x};v:&row;w<W>:{->v:&v};r:f(&w)",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let cells = &checker.reference_cells[&id(&checker, "r")];
        assert!(cells.complete, "{source}");
        assert_eq!(
            cells.places,
            BTreeSet::from([(id(&checker, "v"), vec![])]),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn returned_record_carriers_keep_projected_stored_and_unknown_candidates() {
    for (value, complete) in [("&v", true), ("{->&v}", false)] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;<W>:<{{a<&R>;b<& &R>}}>;f<& &R>:(p<&W>){{->p.&a}};x:=false;row<R>:{{->c:&x}};v:&row;w<W>:{{->a:&row;->b:{value}}};r:f(&w)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "r")];
        assert_eq!(cells.complete, complete);
        let mut expected = BTreeSet::from([(id(&checker, "w"), vec![0])]);
        if complete {
            expected.insert((id(&checker, "v"), vec![]));
        }
        assert_eq!(cells.places, expected);
    }
}

#[test]
pub(crate) fn nullable_terminal_records_keep_carrier_locations() {
    for value in ["{->c:&x}", "null"] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;<M>:<R><null>;f<& &M>:(p<& &M>){{->p}};x:=false;row<M>:{value};v:&row;r:f(&v)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "r")];
        assert!(cells.complete);
        assert_eq!(cells.places, BTreeSet::from([(id(&checker, "v"), vec![])]));
    }
}

#[test]
pub(crate) fn record_carrier_returns_keep_lifetimes_and_type_depth_limits() {
    let source = "<R>:<{c<&boolean>}>;f<& &R>:(p<& &R>){local:*p;->&local};x:=false;row<R>:{->c:&x};v:&row;r:f(&v)";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, "x:=false;row:{->c:&x};v:&row");
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let mut ty = value.ty.clone();
    for _ in 0..=super::super::MAX_CELL_DEPTH {
        ty = Type::Reference(Box::new(ty));
    }
    let error = checker.shared_cell_depth(&ty, value).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("type depth"));
}
