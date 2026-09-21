use crate::ast::{Span, StmtKind};
use crate::check::{
    Checker,
    required::{MAX_STEPS, MAX_TYPES},
};

pub(crate) fn call(source: &str) -> crate::ast::Expr {
    let block = crate::parser::parse(source).unwrap();
    let StmtKind::Bind { value, .. } = block.stmts.into_iter().last().unwrap().kind else {
        panic!()
    };
    value
}

pub(crate) fn checker() -> Checker {
    let mut checker = Checker::new();
    let block = crate::parser::parse(r#"p:@"proof""#).unwrap();
    checker.stmt(&block.stmts[0]).unwrap();
    checker
}

#[test]
pub(crate) fn pending_arguments_charge_calls_and_written_types_once() {
    for (ty, expected) in [
        ("uint32", (2, 1)),
        ("never", (2, 1)),
        ("p.Result", (2, 1)),
        ("Type", (2, 1)),
        ("uint8[2]", (4, 2)),
        ("{n<uint32>}", (4, 3)),
    ] {
        let expr = call(&format!("r:p.can_copy<{ty}>()"));
        let mut checker = checker();
        checker
            .mode_root(expr.span, true, |checker| {
                assert_eq!(checker.pending_query(&expr)?, Some(0));
                let work = checker.type_work.as_ref().unwrap();
                assert!(work.ordinary);
                assert_eq!((work.logical.steps, work.logical.types), expected, "{ty}");
                Ok(())
            })
            .unwrap();
        assert_eq!(checker.queries.len(), 1);
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn pending_argument_limits_share_outer_roots_and_restore_modes() {
    let expr = call("r:p.can_copy<uint32>()");
    let root = Span::new(100, 150);
    for (steps, types, passes) in [
        (MAX_STEPS - 2, MAX_TYPES - 1, true),
        (MAX_STEPS - 1, 0, false),
        (0, MAX_TYPES, false),
    ] {
        let mut checker = checker();
        let result = checker.mode_root(root, true, |checker| {
            let budget = &mut checker.type_work.as_mut().unwrap().logical;
            budget.steps = steps;
            budget.types = types;
            checker.pending_query(&expr)
        });
        assert_eq!(result.is_ok(), passes);
        assert_eq!(checker.queries.len(), usize::from(passes));
        assert_eq!(checker.query_budgets.len(), usize::from(passes));
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, root);
        }
        assert!(checker.type_work.is_none());
        checker.pending_query(&expr).unwrap();
    }
}

#[test]
pub(crate) fn pending_arguments_preserve_modes_and_first_source_errors() {
    for (ty, code, token) in [
        ("uint8[({->2})]", "B001", "({->2})"),
        ("uint8[1/0]", "E107", "1/0"),
        ("Missing", "E202", "Missing"),
        ("(uint32)->uint32", "B001", "<(uint32)->uint32>"),
    ] {
        let source = format!("r:p.can_copy<{ty}>()");
        let expr = call(&source);
        let mut checker = checker();
        let error = checker.pending_query(&expr).unwrap_err();
        assert_eq!(error.code, code, "{source}");
        assert_eq!(&source[error.span.start..error.span.end], token);
        assert!(checker.queries.is_empty());
        assert!(checker.query_budgets.is_empty());
        assert!(checker.type_work.is_none());
    }
    for source in [
        "r:p.can_copy<({-><uint8[({->2})]>})>()",
        "r:p.can_copy<uint8[2]>()",
    ] {
        let mut checker = checker();
        checker.pending_query(&call(source)).unwrap();
        assert!(checker.type_work.is_none());
    }
    let expr = call("r:p.can_copy<uint8[({->2})]>()");
    let mut checker = checker();
    checker
        .required_root(expr.span, |checker| {
            checker.pending_query(&expr)?;
            assert!(!checker.type_work.as_ref().unwrap().ordinary);
            Ok(())
        })
        .unwrap();
}

#[test]
pub(crate) fn pending_argument_arity_checks_precede_constructor_work() {
    for source in [
        "r:p.can_copy<Missing,uint32>()",
        "r:p.can_copy<uint8[1/0]>(missing)",
    ] {
        let expr = call(source);
        let mut checker = checker();
        checker
            .required_root(expr.span, |checker| {
                let error = checker.pending_query(&expr).unwrap_err();
                assert_eq!(error.code, "E212");
                let work = checker.type_work.as_ref().unwrap();
                assert_eq!((work.logical.steps, work.logical.types), (0, 0));
                assert!(checker.queries.is_empty());
                assert!(checker.query_budgets.is_empty());
                Ok(())
            })
            .unwrap();
    }
}

#[test]
pub(crate) fn pending_budgets_keep_shared_root_tail_work_and_copy_identity() {
    let block =
        crate::parser::parse("r:p.can_copy<uint32>();copy:r;second:p.can_copy<never>()").unwrap();
    let root = Span::new(100, 150);
    let mut checker = checker();
    checker
        .mode_root(root, true, |checker| {
            checker
                .type_work
                .as_mut()
                .unwrap()
                .logical
                .charge_all(3, 2, 5)?;
            checker.stmt(&block.stmts[0])?;
            assert_eq!(checker.queries[0].root, 0);
            assert!(checker.query_budgets[0].is_none());
            checker.stmt(&block.stmts[1])?;
            assert_eq!(checker.queries.len(), 1);
            assert!(matches!(
                checker.value("copy", root)?,
                crate::check::Value::Pending(0)
            ));
            checker.required_root(block.stmts[2].span, |checker| {
                checker.stmt(&block.stmts[2])?;
                Ok(())
            })?;
            assert_eq!(checker.queries[1].root, 0);
            assert!(checker.query_budgets[0].is_none());
            checker
                .type_work
                .as_mut()
                .unwrap()
                .logical
                .charge_all(7, 4, 9)
        })
        .unwrap();
    assert!(checker.type_work.is_none());
    assert_eq!(checker.query_budgets.len(), 1);
    let budget = checker.query_budgets[0].as_ref().unwrap();
    assert_eq!(budget.root, root);
    assert_eq!((budget.steps, budget.types, budget.slots), (18, 8, 14));
    assert!(budget.failure.is_none());
}

#[test]
pub(crate) fn pending_budgets_keep_independent_calls_separate_and_skip_unused_roots() {
    let expr = call("r:p.can_copy<uint32>()");
    let mut checker = checker();
    checker
        .required_root(expr.span, |checker| {
            checker.type_work.as_mut().unwrap().logical.charge(20, 10)
        })
        .unwrap();
    assert!(checker.query_budgets.is_empty());
    for id in 0..3 {
        assert_eq!(checker.pending_query(&expr).unwrap(), Some(id));
        assert_eq!(checker.queries[id].root, id);
        let budget = checker.query_budgets[id].as_ref().unwrap();
        assert_eq!(budget.root, expr.span);
        assert_eq!((budget.steps, budget.types), (2, 1));
        assert!(budget.failure.is_none());
    }
}

#[test]
pub(crate) fn pending_budgets_retain_tail_failures_and_restore_next_root() {
    let expr = call("r:p.can_copy<uint32>()");
    let root = Span::new(100, 150);
    let mut checker = checker();
    let error = checker
        .mode_root(root, true, |checker| {
            checker.pending_query(&expr)?;
            let budget = &mut checker.type_work.as_mut().unwrap().logical;
            budget.charge(MAX_STEPS - 2, 0)?;
            assert_eq!(budget.charge(1, 0).unwrap_err().code, "E220");
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, "E220");
    assert_eq!(error.span, root);
    let budget = checker.query_budgets[0].as_ref().unwrap();
    assert_eq!(budget.steps, MAX_STEPS);
    assert_eq!(budget.failure.as_ref().unwrap().span, root);
    assert_eq!(checker.queries[0].unsupported(budget).code, "E220");
    assert!(checker.type_work.is_none());
    checker.pending_query(&expr).unwrap();
    let budget = checker.query_budgets[1].as_ref().unwrap();
    assert_eq!((budget.steps, budget.types), (2, 1));
    assert_eq!(checker.queries[1].unsupported(budget).code, "B001");
}

#[test]
pub(crate) fn pending_recognition_does_not_construct_arguments_or_reserve_queries() {
    for source in [
        "r:p.can_copy<uint32>()",
        "r:(p.can_copy<uint8[1/0]>())",
        "r:p.can_copy<Missing>()",
    ] {
        let expr = call(source);
        let mut checker = checker();
        checker
            .construction_root(expr.span, |checker| {
                assert!(matches!(
                    checker.pending_form(&expr)?,
                    Some(super::Pending::Call { .. })
                ));
                let work = checker.type_work.as_ref().unwrap();
                assert_eq!((work.logical.steps, work.logical.types), (0, 0));
                assert!(checker.queries.is_empty());
                assert!(checker.query_budgets.is_empty());
                Ok(())
            })
            .unwrap();
        assert!(checker.type_work.is_none());
    }
    let mut checker = checker();
    for source in ["r:7", "r:p.revision", "r:p.can_copy<uint32>(missing)"] {
        let result = checker
            .pending_form(&call(source))
            .map(|form| form.is_some());
        if source.contains("missing") {
            assert_eq!(result.unwrap_err().code, "E212");
        } else {
            assert!(!result.unwrap());
        }
        assert!(checker.type_work.is_none());
        assert!(checker.queries.is_empty());
    }
}
