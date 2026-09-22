use crate::check::dependencies::tests::statements;
use crate::check::{Checker, Value};
use std::collections::BTreeSet;

pub(crate) fn id(checker: &Checker, name: &str) -> usize {
    let Value::Local { id, .. } = checker.scopes.last().unwrap().values[name] else {
        panic!()
    };
    id
}

#[test]
pub(crate) fn whole_record_replacements_merge_owners_without_changing_prior_copies() {
    for assignment in ["row={->r:&y}", "|condition|row={->r:&y}"] {
        let source = format!(
            "x:=false;y:=true;condition:=false;row:={{->r:&x}};old:row;{assignment};r:row.r;s:old.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        let s = id(&checker, "s");
        assert_eq!(checker.pointees[&r].roots, BTreeSet::from([0, 1]));
        assert_eq!(checker.pointees[&s].roots, BTreeSet::from([0]));
        checker.mark_derived(1);
        assert!(checker.derived_local(r));
        assert!(!checker.derived_local(s));
    }
}

#[test]
pub(crate) fn whole_record_replacements_preserve_incomplete_fields() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "x:=false;y:=true;row:={->r:{->&x}};row={->r:&y};r:row.r",
    );
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(!origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([1]));
}

#[test]
pub(crate) fn invalid_whole_record_replacements_keep_prior_metadata() {
    let mut checker = Checker::new();
    statements(&mut checker, "x:=false;row:={->r:&x}");
    let block = crate::parser::parse("row=7").unwrap();
    assert_eq!(checker.stmt(&block.stmts[0]).unwrap_err().code, "E207");
    let origins = &checker.record_pointees[&id(&checker, "row")][&0];
    assert!(origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([0]));
}

#[test]
pub(crate) fn field_replacements_merge_only_selected_origins_and_preserve_copies() {
    for assignment in ["row.a=&y", "|condition|row.a=&y"] {
        let source = format!(
            "x:=false;y:=true;z:=false;condition:=false;row:{{->a:=&x;->b:=&z}};old:row;{assignment};r:row.a;s:old.a;t:row.b"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        assert_eq!(
            checker.pointees[&id(&checker, "r")].roots,
            BTreeSet::from([0, 1])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "s")].roots,
            BTreeSet::from([0])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "t")].roots,
            BTreeSet::from([2])
        );
        checker.mark_derived(1);
        assert!(checker.derived_local(id(&checker, "r")));
        assert!(!checker.derived_local(id(&checker, "s")));
        assert!(!checker.derived_local(id(&checker, "t")));
    }
}

#[test]
pub(crate) fn unknown_field_replacements_keep_known_owners_but_not_completeness() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "x:=false;y:=true;row:{->r:=&x};row.r={->&y};r:row.r",
    );
    let origins = &checker.pointees[&id(&checker, "r")];
    assert_eq!(origins.roots, BTreeSet::from([0]));
    assert!(!origins.complete);
}

#[test]
pub(crate) fn invalid_field_replacements_keep_prior_metadata() {
    for (prefix, code) in [
        ("x:=false;row:{->r:&x}", "E305"),
        ("x:=false;row:{->r:=&x}", "E207"),
    ] {
        let mut checker = Checker::new();
        statements(&mut checker, prefix);
        let block = crate::parser::parse("row.r=7").unwrap();
        assert_eq!(checker.stmt(&block.stmts[0]).unwrap_err().code, code);
        let origins = &checker.record_pointees[&id(&checker, "row")][&0];
        assert!(origins.complete);
        assert_eq!(origins.roots, BTreeSet::from([0]));
    }
}
