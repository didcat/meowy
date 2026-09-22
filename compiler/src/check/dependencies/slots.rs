use crate::check::Checker;
use std::collections::BTreeSet;

pub(crate) fn checked(source: &str) -> (Checker, crate::hir::Program) {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let block = crate::parser::parse(source).unwrap();
    let body = checker.block(&block, None, None).unwrap();
    let program = crate::hir::Program {
        body,
        functions: Vec::new(),
        locals: checker.locals.clone(),
    };
    checker.proofs.conditions = checker.guards.clone();
    checker.proofs.tags = checker.tags.clone();
    (checker, program)
}

pub(crate) fn check(source: &str) -> Checker {
    let (mut checker, program) = checked(source);
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    crate::loans::check(&program, &facts, &checker.proofs, &mut checker.flow).unwrap();
    checker
}

#[test]
pub(crate) fn emitted_scalar_references_keep_origins_through_reads() {
    let source =
        "seed:false;x:=seed;p:@\"proof\";row:{->r:&x;flag:*r;|flag|q:p.can_copy<uint32>()}";
    let checker = check(source);
    let (id, alias) = checker
        .proofs
        .aliases
        .iter()
        .find(|(_, alias)| alias.field == "r")
        .unwrap();
    assert_eq!(checker.pointees[&alias.root].roots, BTreeSet::from([1]));
    assert!(checker.derived_local(1));
    assert!(checker.derived_local(*id));
    assert!(checker.queries[0].control);
}

#[test]
pub(crate) fn sibling_reference_slots_merge_origins_and_keep_prior_copies() {
    let source = "seed:false;x:=false;y:=true;c:=false;p:@\"proof\";row:'out{|c|{'out -> r:=&x;old:r;r=&y};|!c|{'out -> r:=&x;|*r|q:p.can_copy<uint32>()}}";
    let mut checker = check(source);
    let aliases = checker
        .proofs
        .aliases
        .iter()
        .filter(|(_, alias)| alias.field == "r")
        .map(|(id, alias)| (*id, alias.root))
        .collect::<Vec<_>>();
    assert_eq!(aliases.len(), 2);
    assert_eq!(aliases[0].1, aliases[1].1);
    let root = aliases[0].1;
    assert_eq!(checker.pointees[&root].roots, BTreeSet::from([1, 2]));
    assert!(!checker.pointees.contains_key(&aliases[1].0));
    let copy = root + 1;
    assert_eq!(checker.pointees[&copy].roots, BTreeSet::from([1]));
    checker.mark_derived(2);
    assert!(aliases.iter().all(|(id, _)| checker.derived_local(*id)));
    assert!(!checker.derived_local(copy));
}

#[test]
pub(crate) fn exclusive_slot_stores_keep_origins_without_bypassing_carrier_gates() {
    let source = "seed:false;x:=false;y:=true;c:=false;row:'out{|c|{'out -> r:&!x};|!c|{'out -> r:&!y;*r=seed}}";
    let (mut checker, program) = checked(source);
    assert!(checker.derived_local(1));
    assert!(checker.derived_local(2));
    let errors = crate::borrow::check(&program, &mut checker.flow, &checker.proofs)
        .err()
        .unwrap();
    assert_eq!(errors[0].code, "B001");
    assert!(errors[0].message.contains("exclusive reference carriers"));
}

#[test]
pub(crate) fn incomplete_sibling_origins_still_gate_marked_stores() {
    let source = "seed:false;x:=false;y:=true;c:=false;row:'out{|c|{'out -> r:{->&!x}};|!c|{'out -> r:&!y;*r=seed}}";
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let block = crate::parser::parse(source).unwrap();
    let error = checker.block(&block, None, None).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("indirect store origins"));
    assert!(!checker.derived_local(1));
    assert!(!checker.derived_local(2));
}
