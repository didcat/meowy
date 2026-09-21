use super::tests::statements;
use crate::check::Checker;
use crate::hir::Stmt;

#[test]
pub(crate) fn scalar_writes_preserve_data_and_lexical_control_dependencies() {
    for assignment in ["value=flag", "|flag|value=false", "|!flag|value=true"] {
        let mut checker = Checker::new();
        statements(&mut checker, "flag:false;value:=true;plain:=false");
        checker.derived.insert(0);
        statements(&mut checker, assignment);
        statements(&mut checker, "copy:value;again:copy;plain=true");
        assert!(checker.derived_local(1));
        assert!(checker.derived_local(3));
        assert!(checker.derived_local(4));
        assert!(!checker.derived_local(2));
        assert!(!checker.control);
    }
}

#[test]
pub(crate) fn scalar_write_dependencies_keep_query_guards_opaque() {
    use crate::flow::{FALSE, TRUE};

    let mut checker = Checker::new();
    statements(&mut checker, "flag:false;value:=true;p:@\"proof\"");
    checker.derived.insert(0);
    statements(&mut checker, "|flag|value=false");
    let stmts = statements(&mut checker, "|value|r:p.can_copy<uint32>()");
    let Stmt::If { condition, .. } = &stmts[0] else {
        panic!()
    };
    let guard = checker.guard(condition);
    assert_ne!(guard, TRUE);
    assert_ne!(guard, FALSE);
    assert!(checker.queries[0].control);
    let error =
        crate::check::queries::finish(&checker.queries, &checker.query_budgets).unwrap_err();
    assert_eq!(error.code, "E225");
}

#[test]
pub(crate) fn independent_overwrites_do_not_clear_conservative_write_marks() {
    let mut checker = Checker::new();
    statements(&mut checker, "flag:true;value:=false");
    checker.derived.insert(0);
    statements(&mut checker, "value=flag;value=false;copy:value");
    assert!(checker.derived_local(1));
    assert!(checker.derived_local(2));
}

#[test]
pub(crate) fn scalar_write_errors_do_not_mark_unchanged_storage() {
    for (prefix, assignment, code) in [
        ("flag:true;value:false", "value=flag", "E305"),
        ("flag:true;value:=7", "value=flag", "E207"),
    ] {
        let mut checker = Checker::new();
        statements(&mut checker, prefix);
        checker.derived.insert(0);
        let block = crate::parser::parse(assignment).unwrap();
        let error = checker.stmt(&block.stmts[0]).unwrap_err();
        assert_eq!(error.code, code);
        assert!(!checker.derived_local(1));
    }
}
