use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn borrowed_record_cell_projections_keep_exact_field_locations() {
    for tail in [
        "r:f(&row)",
        "v:&row;copy:v;r:f(copy)",
        "outer:{->inner:row};r:f(outer.&inner)",
    ] {
        let source = format!(
            "<R>:<{{c<&boolean>}}>;f<& &boolean>:(p<&R>){{->p.&c}};x:=false;row<R>:{{->c:&x}};{tail};out:*r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        assert!(checker.reference_cells[&r].complete);
        let (root, path) = if tail.contains("outer") {
            (id(&checker, "outer"), vec![0, 0])
        } else {
            (id(&checker, "row"), vec![0])
        };
        assert_eq!(
            checker.reference_cells[&r].places,
            BTreeSet::from([(root, path)])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "out")].roots,
            BTreeSet::from([id(&checker, "x")])
        );
    }
}

#[test]
pub(crate) fn borrowed_records_combine_projected_cells_and_stored_carriers() {
    let source = "<N>:<{c<& & &boolean>}>;<R>:<{own<&boolean>;inner<N>}>;f<& &boolean>:(p<&R>){->p.&own};x:=false;y:=true;a:&y;b:&a;row<R>:{->own:&x;->inner:{->c:&b}};r:f(&row);out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "row"), vec![1]), (id(&checker, "a"), vec![])])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
}

#[test]
pub(crate) fn borrowed_record_unknown_carriers_preserve_known_projections() {
    let source = "<R>:<{own<&boolean>;c<& &boolean>}>;f<& &boolean>:(p<&R>){->p.&own};x:=false;y:=true;a:&y;row<R>:{->own:&x;->c:{->&a}};r:f(&row);out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(!cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "row"), vec![1])])
    );
    assert!(!checker.pointees[&id(&checker, "out")].complete);
}

#[test]
pub(crate) fn borrowed_record_returned_cells_follow_nullable_stored_carriers() {
    for (value, names) in [("{->c:&a}", vec!["a", "b"]), ("null", vec!["b"])] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}>;<M>:<R><null>;f<& &boolean>:(p<&M>,q<& &boolean>){{v:*p;|v<R>|->v.c;|v<null>|->q}};x:=false;y:=true;a:&x;b:&y;row<M>:{value};r:f(&row,&b)"
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
