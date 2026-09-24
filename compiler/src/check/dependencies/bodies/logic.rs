use super::{
    Fact, Role,
    relations::{at, link},
    tests::check,
};
use crate::check::Checker;

#[test]
pub(crate) fn logical_body_facts_keep_condition_and_selected_operand_roles() {
    for (op, fact, role) in [("&&", Fact::And, Role::Then), ("||", Fact::Or, Role::Else)] {
        let source = format!("left:=false;right:=true;out:left{op}right");
        crate::compile(&source).unwrap();
        let (checker, root) = check(&source);
        let body = &checker.bodies[&root.id];
        let logic = at(body, fact);
        link(body, logic, at(body, Fact::Bind(2)), Role::Data);
        link(body, at(body, Fact::Read(0)), logic, Role::Condition);
        link(body, at(body, Fact::Read(1)), logic, role);
        assert_eq!(body.storage[logic], None);
    }
}

#[test]
pub(crate) fn logical_body_facts_keep_nested_rhs_effects_and_independent_successors() {
    let source = "flag:=false;x:=0;out:flag&&{x=1;->true};plain:x";
    crate::compile(source).unwrap();
    let (checker, root) = check(source);
    let body = &checker.bodies[&root.id];
    let logic = at(body, Fact::And);
    let (index, child) = body
        .facts
        .iter()
        .enumerate()
        .find_map(|(index, (fact, _))| match fact {
            Fact::Block(id) => Some((index, *id)),
            _ => None,
        })
        .unwrap();
    link(body, index, logic, Role::Then);
    assert_eq!(
        checker.bodies[&child].links[at(&checker.bodies[&child], Fact::Write(1))],
        None
    );
    let plain = at(body, Fact::Bind(3));
    assert_eq!(body.links[plain], None);
    link(body, at(body, Fact::Read(1)), plain, Role::Data);
    assert!(checker.derived.is_empty());
}

#[test]
pub(crate) fn logical_body_facts_preserve_both_inputs_under_derived_conditions() {
    let source = "a:false;b:true;c:false;out:a&&(b||c)";
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let block = crate::parser::parse(source).unwrap();
    let root = checker.block(&block, None, None).unwrap();
    let body = &checker.bodies[&root.id];
    let and = at(body, Fact::And);
    let or = at(body, Fact::Or);
    link(body, or, and, Role::Then);
    link(body, at(body, Fact::Read(0)), and, Role::Condition);
    link(body, at(body, Fact::Read(1)), or, Role::Condition);
    link(body, at(body, Fact::Read(2)), or, Role::Else);
    assert!(checker.derived_local(3));
    assert!(!checker.derived_local(1));
    assert!(!checker.derived_local(2));
}

#[test]
pub(crate) fn logical_body_facts_keep_skipped_reads_and_ordinary_diagnostics() {
    for source in ["flag:true;out:false&&flag", "flag:false;out:true||flag"] {
        let (checker, root) = check(source);
        let body = &checker.bodies[&root.id];
        assert!(body.links[at(body, Fact::Read(0))].is_some());
    }
    for (source, code) in [
        ("out:false&&{x<boolean>:1;->true}", "E207"),
        (
            "flag:=false;x:=1;r:&x;out:flag&&{x=2;->true};copy:*r",
            "E302",
        ),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
}
