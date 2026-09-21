use super::*;
use crate::check::Constant;

pub(crate) fn statements(checker: &mut Checker, source: &str) -> Vec<Stmt> {
    crate::parser::parse(source)
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
