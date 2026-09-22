use super::tests::statements;
use crate::check::Checker;
use std::collections::BTreeSet;

#[test]
pub(crate) fn mutable_references_keep_prior_copies_and_possible_owners() {
    let mut checker = Checker::new();
    statements(&mut checker, "x:=false;y:=true;r:=&x;old:r;r=&y;copy:r");
    assert_eq!(checker.pointees[&2].roots, BTreeSet::from([0, 1]));
    assert_eq!(checker.pointees[&3].roots, BTreeSet::from([0]));
    assert_eq!(checker.pointees[&4].roots, BTreeSet::from([0, 1]));
    checker.mark_derived(1);
    assert!(checker.derived_local(2));
    assert!(!checker.derived_local(3));
    assert!(checker.derived_local(4));
}

#[test]
pub(crate) fn retargeted_indirect_stores_mark_all_retained_owners() {
    for retarget in ["r=&!y", "|condition|r=&!y"] {
        let source = format!(
            "seed:false;x:=false;y:=true;r:=&!x;condition:=false;{retarget};*r=seed;flag:*r;p:@\"proof\";|flag|q:p.can_copy<uint32>()"
        );
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let block = crate::parser::parse(&source).unwrap();
        let body = checker.block(&block, None, None).unwrap();
        assert!(checker.derived_local(1));
        assert!(checker.derived_local(2));
        assert_eq!(checker.pointees[&3].roots, BTreeSet::from([1, 2]));
        assert!(checker.queries[0].control);
        let program = crate::hir::Program {
            body,
            functions: Vec::new(),
            locals: checker.locals.clone(),
        };
        checker.proofs.conditions = checker.guards.clone();
        checker.proofs.tags = checker.tags.clone();
        let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
        crate::loans::check(&program, &facts, &checker.proofs, &mut checker.flow).unwrap();
    }
}

#[test]
pub(crate) fn incomplete_retargeting_cannot_silently_drop_possible_owners() {
    for body in ["r:={->&!x};r=&!y", "r:=&!x;r={->&!y}"] {
        let source = format!("seed:false;x:=false;y:=true;{body};*r=seed");
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let block = crate::parser::parse(&source).unwrap();
        let error = checker.block(&block, None, None).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("indirect store origins"));
        assert!(!checker.derived_local(1));
        assert!(!checker.derived_local(2));
    }
}

#[test]
pub(crate) fn invalid_retargeting_preserves_prior_origin_metadata() {
    let mut checker = Checker::new();
    statements(&mut checker, "x:=false;r:=&x");
    let block = crate::parser::parse("r=7").unwrap();
    assert_eq!(checker.stmt(&block.stmts[0]).unwrap_err().code, "E207");
    assert_eq!(checker.pointees[&1].roots, BTreeSet::from([0]));
    assert!(checker.pointees[&1].complete);
}
