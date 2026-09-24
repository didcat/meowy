use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn nullable_returned_record_views_preserve_storage_even_when_null() {
    for init in ["{->c:&x}", "null"] {
        for tail in ["r:f(&row)", "r:f(f(&row))", "view:f(&row);r:view"] {
            let source = format!(
                "<R>:<{{c<&boolean>}}>;<M>:<R><null>;f<&M>:(p<&M>){{->p}};x:=false;row<M>:{init};{tail}"
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
            assert_eq!(checker.derived_local(r), init != "null");
        }
    }
}

#[test]
pub(crate) fn nullable_returned_views_keep_stored_and_projected_locations() {
    for init in ["{->c:&x}", "null"] {
        for (defs, tail, field) in [
            (
                "<W>:<{v<&M>}>;f<&M>:(p<W>){->p.v}",
                "row<M>:VALUE;r:f({->v:&row})",
                vec![],
            ),
            (
                "<W>:<{inner<M>}>;f<&M>:(p<&W>){->p.&inner}",
                "row<W>:{->inner<M>:VALUE};r:f(&row)",
                vec![0],
            ),
        ] {
            let tail = tail.replace("VALUE", init);
            let source = format!("<R>:<{{c<&boolean>}}>;<M>:<R><null>;{defs};x:=false;{tail}");
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let cells = &checker.reference_cells[&id(&checker, "r")];
            assert!(cells.complete);
            assert_eq!(cells.places, BTreeSet::from([(id(&checker, "row"), field)]));
        }
    }
}

#[test]
pub(crate) fn nullable_returned_record_views_keep_unknown_candidates_incomplete() {
    let source = "<R>:<{c<&boolean>}>;<M>:<R><null>;f<&M>:(a<&M>,b<&M>){->a};x:=false;one<M>:{->c:&x};two<M>:null;r:f(&one,{->&two})";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(!cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "one"), vec![])])
    );
}

#[test]
pub(crate) fn nullable_result_lifetimes_and_heterogeneous_results_remain_checked() {
    let source = "<R>:<{c<&boolean>}>;<M>:<R><null>;f<&M>:(p<&M>){local<M>:null;->&local};x:=false;row<M>:{->c:&x};r:f(&row)";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
    let source = "<A>:<{a<&boolean>}>;<B>:<{b<&boolean>}>;<M>:<A><B>;f<&M>:(p<&M>){->p};x:=false;row<M>:{->a:&x};r:f(&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let cells = checker.reference_cell(value).unwrap();
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "row"), vec![])])
    );
}
