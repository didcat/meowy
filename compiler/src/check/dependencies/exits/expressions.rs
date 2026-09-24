use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};

#[test]
pub(crate) fn later_call_operands_capture_continuation_control_in_temporaries() {
    let source = "f<int32>:(a<int32>,b<&int32>){->a};flag:false;'out{f({|flag|'out.leave();->1},&7)};plain:f(0,&9)";
    crate::compile(source).unwrap();
    let mut ordinary = Checker::new();
    statements(&mut ordinary, source);
    assert!(ordinary.derived.is_empty());
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "f<int32>:(a<int32>,b<&int32>){->a};flag:false",
    );
    checker.mark_derived(id(&checker, "flag"));
    statements(
        &mut checker,
        "'out{f({|flag|'out.leave();->1},&7)};plain:f(0,&9)",
    );
    let owners = checker
        .proofs
        .temporaries
        .keys()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), 2);
    assert!(checker.derived_local(owners[0]));
    assert!(!checker.derived_local(owners[1]));
    assert!(!checker.control);
    assert!(!checker.continuation_control());
}

#[test]
pub(crate) fn index_leave_controls_rhs_temporaries_and_writes() {
    let source =
        "flag:false;items<int32[1]>:=[0];'out{items[{|flag|'out.leave();->1}]=*(&7)};plain:*(&9)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, "flag:false;items<int32[1]>:=[0]");
    checker.mark_derived(id(&checker, "flag"));
    statements(
        &mut checker,
        "'out{items[{|flag|'out.leave();->1}]=*(&7)};plain:*(&9)",
    );
    let owners = checker
        .proofs
        .temporaries
        .keys()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), 2);
    assert!(checker.derived_local(owners[0]));
    assert!(!checker.derived_local(owners[1]));
    assert!(checker.derived_local(id(&checker, "items")));
    assert!(!checker.derived_local(id(&checker, "plain")));
}

#[test]
pub(crate) fn operand_target_joins_clear_availability_control() {
    let source =
        "f<int32>:(a<int32>,b<&int32>){->a};flag:false;out:f('inner{->1;|flag|'inner.leave()},&7)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "f<int32>:(a<int32>,b<&int32>){->a};flag:false",
    );
    checker.mark_derived(id(&checker, "flag"));
    statements(&mut checker, "out:f('inner{->1;|flag|'inner.leave()},&7)");
    let owners = checker
        .proofs
        .temporaries
        .keys()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), 1);
    assert!(!checker.derived_local(owners[0]));
    assert!(checker.derived_local(id(&checker, "out")));
    assert!(!checker.continuation_control());
}

#[test]
pub(crate) fn later_operand_queries_keep_derived_availability() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "f<int32>:(a<int32>,b<int32>){->a};flag:false;p:@\"proof\"",
    );
    checker.mark_derived(id(&checker, "flag"));
    statements(
        &mut checker,
        "'out{f({|flag|'out.leave();->1},{q:p.can_copy<uint32>();->2})};plain:p.can_copy<uint8>()",
    );
    assert!(checker.queries[0].control);
    assert!(!checker.queries[1].control);
    assert_eq!(
        crate::check::queries::finish(&checker.queries, &checker.query_budgets)
            .unwrap_err()
            .code,
        "E225"
    );
}

#[test]
pub(crate) fn expression_continuation_errors_restore_control_and_keep_precedence() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "f<int32>:(a<int32>,b<&int32>){->a};flag:false",
    );
    checker.mark_derived(id(&checker, "flag"));
    let block = crate::parser::parse("'out{f({|flag|'out.leave();->1},false)}").unwrap();
    assert_eq!(checker.stmt(&block.stmts[0]).unwrap_err().code, "E212");
    assert!(!checker.control);
    assert!(!checker.continuation_control());
    assert!(!checker.flow.spend(usize::MAX));
    let block = crate::parser::parse("3").unwrap();
    let crate::ast::StmtKind::Expr(expr) = &block.stmts[0].kind else {
        panic!()
    };
    let error = checker.expression(expr, None).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation expression budget"));
    assert!(!checker.control);
}

#[test]
pub(crate) fn final_expression_type_errors_restore_continuation_state() {
    let mut checker = Checker::new();
    statements(&mut checker, "f<int32>:(a<int32>,b<int32>){->a};flag:false");
    checker.mark_derived(id(&checker, "flag"));
    let block = crate::parser::parse("'out{}").unwrap();
    let crate::ast::StmtKind::Expr(value) = &block.stmts[0].kind else {
        panic!()
    };
    let crate::ast::ExprKind::Block(target) = &value.kind else {
        panic!()
    };
    checker.block_start(target, None, None, false).unwrap();
    let block = crate::parser::parse("f({|flag|'out.leave();->1},2)").unwrap();
    let crate::ast::StmtKind::Expr(expr) = &block.stmts[0].kind else {
        panic!()
    };
    assert_eq!(
        checker
            .expr(expr, Some(&crate::hir::Type::Bool))
            .unwrap_err()
            .code,
        "E207"
    );
    assert!(!checker.control);
    assert!(!checker.continuation_control());
}
