use super::*;
use crate::check::dependencies::{OperationKind, carriers::id, tests::statements};

#[test]
pub(crate) fn short_circuit_controls_mark_rhs_values_writes_and_nested_conditions() {
    for op in ["&&", "||"] {
        let mut checker = checker();
        statements(&mut checker, "flag:false;x:=0");
        statements(
            &mut checker,
            &format!("flag{op}{{x=1;value:3;true&&false;->true}};plain:4"),
        );
        assert!(checker.derived_local(id(&checker, "x")));
        assert!(!checker.derived_local(id(&checker, "plain")));
        assert!(
            checker
                .operations
                .values()
                .filter(|op| op.kind == OperationKind::Write)
                .all(|op| op.control)
        );
        assert!(checker.branch_inputs.values().all(|input| input.control));
        assert!(!checker.control);
    }
}

#[test]
pub(crate) fn short_circuit_controls_reject_query_dependencies_without_leaking_past_joins() {
    for op in ["&&", "||"] {
        let mut checker = checker();
        statements(&mut checker, "flag:false;p:@\"proof\"");
        statements(
            &mut checker,
            &format!("flag{op}{{q:p.can_copy<uint8>();->true}};plain:p.can_copy<uint16>()"),
        );
        assert!(checker.queries[0].control);
        assert!(!checker.queries[1].control);
        assert_eq!(
            crate::check::queries::finish(&checker.queries, &checker.query_budgets)
                .unwrap_err()
                .code,
            "E225"
        );
        assert!(!checker.control);
    }
}

#[test]
pub(crate) fn short_circuit_controls_propagate_outer_exits_but_respect_target_joins() {
    for op in ["&&", "||"] {
        let mut checker = checker();
        statements(&mut checker, "flag:false;p:@\"proof\"");
        statements(
            &mut checker,
            &format!(
                "'out{{flag{op}{{'out.leave()}};q:p.can_copy<uint8>()}};plain:p.can_copy<uint16>()"
            ),
        );
        assert!(checker.queries[0].control);
        assert!(!checker.queries[1].control);
        assert!(!checker.continuation_control());
        let mut checker = super::checker();
        statements(&mut checker, "flag:false;p:@\"proof\"");
        statements(
            &mut checker,
            &format!("flag{op}'inner{{->true;'inner.leave()}};plain:p.can_copy<uint8>()"),
        );
        assert!(!checker.queries[0].control);
        assert!(!checker.control);
        assert!(!checker.continuation_control());
    }
}

#[test]
pub(crate) fn short_circuit_controls_preserve_ordinary_errors_and_restore_checker_control() {
    for (tail, code) in [
        ("flag&&{bad<int32>:false;->true}", "E207"),
        ("flag||missing", "E201"),
        ("flag&&1", "E222"),
    ] {
        let mut checker = checker();
        statements(&mut checker, "flag:false");
        let block = crate::parser::parse(tail).unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, code);
        assert!(!checker.control);
        assert!(!checker.continuation_control());
    }
}

#[test]
pub(crate) fn short_circuit_controls_leave_ordinary_conditions_independent() {
    for op in ["&&", "||"] {
        let source = format!("flag:=false;x:=0;flag{op}{{x=1;true&&false;->true}};plain:4");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        assert!(checker.derived.is_empty());
        assert!(checker.branch_inputs.values().all(|input| !input.control));
        assert!(!checker.control);
    }
}
