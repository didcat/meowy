use super::*;
use crate::check::Constant;

pub(crate) fn statements(checker: &mut Checker, source: &str) -> Vec<Stmt> {
    let offset = checker
        .guards
        .keys()
        .map(|(_, end)| *end)
        .max()
        .unwrap_or(0)
        + 1;
    let source = format!("{}{source}", " ".repeat(offset));
    crate::parser::parse(&source)
        .unwrap()
        .stmts
        .iter()
        .flat_map(|stmt| checker.stmt(stmt).unwrap())
        .collect()
}

#[test]
pub(crate) fn derived_operands_cannot_fold_through_short_circuit_or_copies() {
    for value in ["flag", "!flag", "false&&flag", "true||flag", "flag==flag"] {
        let mut checker = Checker::new();
        statements(&mut checker, "flag:true");
        checker.derived.insert(0);
        let stmts = statements(&mut checker, &format!("copy:{value};again:copy"));
        assert!(checker.derived.contains(&1));
        assert!(checker.derived.contains(&2));
        assert!(checker.bool_inputs[&1].derived);
        assert!(checker.bool_inputs[&2].derived);
        for stmt in stmts {
            let Stmt::Bind { value, .. } = stmt else {
                panic!()
            };
            assert!(checker.derived_expr(&value));
            assert!(checker.constant(&value).is_none());
        }
    }
}

#[test]
pub(crate) fn derived_walk_keeps_skipped_block_and_call_argument_dependencies() {
    for tail in [
        "copy:{|false|{unused:flag};->true}",
        "f<boolean>:(x<boolean>){->true};copy:f(flag)",
        "copy:{->b:flag};again:copy.b",
    ] {
        let mut checker = Checker::new();
        statements(&mut checker, "flag:true");
        checker.derived.insert(0);
        let stmts = statements(&mut checker, tail);
        let Stmt::Bind { id, value } = stmts.last().unwrap() else {
            panic!()
        };
        assert!(checker.derived.contains(id), "{tail}");
        assert!(checker.derived_expr(value), "{tail}");
    }
}

#[test]
pub(crate) fn ordinary_constants_and_fixed_signatures_remain_independent() {
    let mut checker = Checker::new();
    statements(&mut checker, "flag:true");
    checker.derived.insert(0);
    let stmts = statements(&mut checker, "<B>:flag<>;plain<B>:false");
    let Stmt::Bind { id, value } = stmts.last().unwrap() else {
        panic!()
    };
    assert!(!checker.derived.contains(id));
    assert!(matches!(
        checker.constant(value),
        Some(Constant::Bool(false))
    ));
}

#[test]
pub(crate) fn derived_guards_keep_both_paths_without_boolean_correlations() {
    use crate::flow::{FALSE, TRUE};

    let mut checker = Checker::new();
    statements(&mut checker, "flag:true");
    checker.derived.insert(0);
    let stmts = statements(
        &mut checker,
        "|flag|{};|!flag|{};|false&&flag|{};|true||flag|{}",
    );
    let guards = stmts
        .iter()
        .map(|stmt| {
            let Stmt::If { condition, .. } = stmt else {
                panic!()
            };
            let guard = checker.guard(condition);
            assert_ne!(guard, TRUE);
            assert_ne!(guard, FALSE);
            assert_eq!(guard, checker.guard(condition));
            guard
        })
        .collect::<Vec<_>>();
    assert_ne!(checker.flow.and(guards[0], guards[1]), FALSE);
    assert_ne!(checker.flow.or(guards[0], guards[1]), TRUE);
}

#[test]
pub(crate) fn derived_conditions_cannot_hide_loan_conflicts() {
    use crate::hir::Program;

    for condition in ["flag", "!flag", "false&&flag"] {
        let mut checker = Checker::new();
        checker.derived.insert(0);
        let source = format!("flag:false;x:=7;r:&x;|{condition}|x=8;copy:*r");
        let block = crate::parser::parse(&source).unwrap();
        let body = checker.block(&block, None, None).unwrap();
        checker.proofs.conditions = checker.guards.clone();
        let program = Program {
            body,
            functions: Vec::new(),
            locals: checker.locals.clone(),
        };
        let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
        let errors =
            crate::loans::check(&program, &facts, &checker.proofs, &mut checker.flow).unwrap_err();
        assert_eq!(errors[0].code, "E302", "{condition}");
    }
}

#[test]
pub(crate) fn ordinary_guards_keep_constant_and_complementary_paths() {
    use crate::flow::{FALSE, TRUE};

    let mut checker = Checker::new();
    let stmts = statements(&mut checker, "flag:=false;|flag|{};|!flag|{};|false|{}");
    let guards = stmts
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::If { condition, .. } => Some(checker.guard(condition)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(checker.flow.and(guards[0], guards[1]), FALSE);
    assert_eq!(checker.flow.or(guards[0], guards[1]), TRUE);
    assert_eq!(guards[2], FALSE);
}

#[test]
pub(crate) fn derived_complements_cannot_justify_exclusive_emissions() {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let source = "flag:true;n:{|flag|->3;|!flag|->4}";
    let block = crate::parser::parse(source).unwrap();
    let error = checker.block(&block, None, None).unwrap_err();
    assert_eq!(error.code, "E205");
    crate::compile(source).unwrap();
}
