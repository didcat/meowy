use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Type;
use std::collections::BTreeSet;

#[test]
pub(crate) fn deeper_carrier_calls_recover_named_temporary_and_record_owners() {
    for source in [
        "f<&boolean>:(p<& & &boolean>){->**p};x:=false;a:&x;b:&a;r:f(&b)",
        "f<&boolean>:(p<& & & &boolean>){->***p};x:=false;a:&x;b:&a;c:&b;r:f(&c)",
        "f<&boolean>:(p<& & &boolean>){->**p};x:=false;r:f(&(&(&x)))",
        "<R>:<{c<& & &boolean>}>;f<&boolean>:(p<R>){->**(p.c)};x:=false;a:&x;b:&a;r:f({->c:&b})",
        "f<&boolean>:(p<& & &boolean[2]>){->&((**p)[1])};x<boolean[2]>:=[false,true];a:&x;b:&a;r:f(&b)",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let r = id(&checker, "r");
        let x = id(&checker, "x");
        assert!(checker.pointees[&r].complete, "{source}");
        assert_eq!(checker.pointees[&r].roots, BTreeSet::from([x]), "{source}");
        checker.mark_derived(x);
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn deeper_carrier_retargets_keep_old_and_new_owners() {
    let source =
        "f<&boolean>:(p<& & &boolean>){->**p};x:=false;y:=true;a:&x;b:&y;c:=&a;c=&b;r:f(&c)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(origins.complete);
    assert_eq!(
        origins.roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
}

#[test]
pub(crate) fn unknown_intermediate_cells_preserve_known_owners() {
    let source =
        "f<&boolean>:(p<& & &boolean>){->**p};x:=false;y:=true;a:&x;b:&y;c:=&a;c={->&b};r:f(&c)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(!origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
}

#[test]
pub(crate) fn shared_carrier_type_traversal_keeps_depth_work_and_mode_limits() {
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, "x:=false;p:&x");
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let mut ty = Type::Reference(Box::new(Type::Bool));
    for _ in 0..super::MAX_CELL_DEPTH {
        ty = Type::Reference(Box::new(ty));
    }
    assert_eq!(
        checker.call_shared_view(&ty, value).unwrap().unwrap().1,
        super::MAX_CELL_DEPTH
    );
    ty = Type::Reference(Box::new(ty));
    let error = checker.call_shared_view(&ty, value).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("cell depth"));
    let ty = Type::Reference(Box::new(Type::Exclusive(Box::new(Type::Bool))));
    assert!(checker.call_shared_view(&ty, value).unwrap().is_none());
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(
        checker.call_shared_view(&ty, value).err().unwrap().code,
        "B001"
    );
}
