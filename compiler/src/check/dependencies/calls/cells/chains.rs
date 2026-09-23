use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Type;
use std::collections::BTreeSet;

#[test]
pub(crate) fn deeper_returned_carriers_keep_cells_through_nested_calls_and_copies() {
    for tail in [
        "r:f(&b);out:**r",
        "r:f(f(&b));copy:r;out:**copy",
        "out:**(f(&b))",
    ] {
        let source = format!("f<& & &boolean>:(p<& & &boolean>){{->p}};x:=false;a:&x;b:&a;{tail}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = id(&checker, "out");
        let x = id(&checker, "x");
        assert!(checker.pointees[&out].complete);
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn deeper_inputs_supply_all_compatible_inner_return_cells() {
    let source = "f<& &boolean>:(p<& & &boolean>,q<& &boolean>){->q};x:=false;y:=true;a:&x;b:&a;c:&y;r:f(&b,&c);out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(cells.complete);
    assert_eq!(
        cells.places,
        BTreeSet::from([(id(&checker, "a"), vec![]), (id(&checker, "c"), vec![])])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
}

#[test]
pub(crate) fn unknown_deeper_return_candidates_keep_known_locations_incomplete() {
    let source = "f<& &boolean>:(p<& & &boolean>,q<& &boolean>){->q};x:=false;y:=true;a:&x;b:{->&a};c:&y;r:f(&b,&c);out:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let cells = &checker.reference_cells[&id(&checker, "r")];
    assert!(!cells.complete);
    assert_eq!(cells.places, BTreeSet::from([(id(&checker, "c"), vec![])]));
    assert!(!checker.pointees[&id(&checker, "out")].complete);
}

#[test]
pub(crate) fn returned_chain_type_depth_modes_and_work_remain_bounded() {
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, "x:=false;p:&x");
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let mut ty = Type::Bool;
    for _ in 0..=super::MAX_CELL_DEPTH {
        ty = Type::Reference(Box::new(ty));
    }
    assert_eq!(
        checker.shared_cell_depth(&ty, value).unwrap(),
        Some(super::MAX_CELL_DEPTH + 1)
    );
    ty = Type::Reference(Box::new(ty));
    let error = checker.shared_cell_depth(&ty, value).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("type depth"));
    let ty = Type::Reference(Box::new(Type::Exclusive(Box::new(Type::Bool))));
    assert_eq!(checker.shared_cell_depth(&ty, value).unwrap(), None);
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(
        checker.shared_cell_depth(&ty, value).err().unwrap().code,
        "B001"
    );
}
