use super::tests::statements;
use crate::check::{Checker, Value};
use std::collections::BTreeSet;

pub(crate) fn id(checker: &Checker, name: &str) -> usize {
    let Value::Local { id, .. } = checker.scopes.last().unwrap().values[name] else {
        panic!()
    };
    id
}

#[test]
pub(crate) fn copied_temporary_references_keep_pointees_not_temporary_storage() {
    for source in ["x:=false;r:*(&(&x))", "x:=false;r:*(&(*(&(&x))))"] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let x = id(&checker, "x");
        let r = id(&checker, "r");
        assert_eq!(checker.pointees[&r].roots, BTreeSet::from([x]));
        assert!(checker.pointees[&r].complete);
        assert!(!checker.proofs.temporaries.contains_key(&x));
        checker.mark_derived(x);
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn copied_temporary_records_and_direct_fields_keep_external_origins() {
    for source in [
        "x:=false;copy:*(&{->view:&x});r:copy.view",
        "x:=false;copy:*(&{->inner:{->view:&x}});r:copy.inner.view",
        "x:=false;r:(*(&{->view:&x})).view",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
        assert!(origins.complete);
    }
}

#[test]
pub(crate) fn temporary_carriers_preserve_unknown_calls_and_expiry() {
    let source = "f<&boolean>:(p<&boolean>){->p};x:=false;r:*(&(f({->&x})))";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.pointees[&id(&checker, "r")].complete);
    let error = crate::compile("escaped:&(&false);copy:*escaped")
        .unwrap_err()
        .remove(0);
    assert_eq!(error.code, "E303");
}
