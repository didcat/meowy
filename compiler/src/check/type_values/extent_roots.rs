use crate::ast::{Expr, Span, StmtKind};
use crate::check::{Checker, Value, inputs::Input, required::MAX_STEPS};
use crate::hir::Type;

pub(crate) fn expression(source: &str) -> Expr {
    let block = crate::parser::parse(&format!("v:{source}")).unwrap();
    let StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
        panic!()
    };
    value.clone()
}

#[test]
pub(crate) fn ordinary_extents_reset_roots_and_restore_errors() {
    let mut checker = Checker::new();
    for source in ["1+2", "(((3)))", "4-1"] {
        assert_eq!(checker.list_extent(&expression(source)).unwrap(), 3);
        assert!(checker.type_work.is_none());
        assert!(!checker.required);
    }
    for (source, code) in [
        ("1/0", "E107"),
        ("-1", "E104"),
        ("1.5", "E104"),
        ("missing", "E201"),
    ] {
        let expr = expression(source);
        let reach = checker.reach;
        let error = checker.list_extent(&expr).unwrap_err();
        assert_eq!(error.code, code, "{source}");
        assert_eq!(checker.reach, reach);
        assert!(!checker.required);
        assert!(checker.type_work.is_none());
        assert_eq!(checker.list_extent(&expression("2")).unwrap(), 2);
    }
}

#[test]
pub(crate) fn ordinary_extent_evaluation_charges_groups_and_limits() {
    let root = Span::new(100, 150);
    for source in ["1+2", "(((1)))+((2))"] {
        for remaining in [2, 3, 4] {
            let mut checker = Checker::new();
            let result = checker.required_root(root, |checker| {
                let work = checker.type_work.as_mut().unwrap();
                work.ordinary = true;
                work.logical.steps = MAX_STEPS - remaining;
                let result = checker.list_extent(&expression(source));
                assert!(!checker.required);
                result
            });
            assert_eq!(result.is_ok(), remaining >= 3);
            if let Err(error) = result {
                assert_eq!(error.code, "E220");
                assert_eq!(error.span, root);
            }
            assert!(checker.type_work.is_none());
        }
    }
}

#[test]
pub(crate) fn ordinary_extents_keep_forms_and_initializer_gates() {
    for source in ["{->2}", "m.n", "f()"] {
        let mut checker = Checker::new();
        let error = checker.list_extent(&expression(source)).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(checker.type_work.is_none());
    }
    crate::compile("n:2;v<int32[n]>:[7]").unwrap();
    crate::compile("n:{->2};v<int32[n]>:[7]").unwrap();
    let mut checker = Checker::new();
    let ty = Type::Int {
        bits: 32,
        signed: true,
    };
    let id = checker.local(ty.clone());
    checker
        .declare(
            "n",
            Value::Local {
                id,
                ty,
                mutable: false,
                owner: 0,
                constant: None,
            },
            Span::new(0, 1),
        )
        .unwrap();
    checker.inputs.insert(
        id,
        Input {
            value: Some(2),
            error: None,
            work: 1,
        },
    );
    let expr = expression("n");
    assert_eq!(checker.list_extent(&expr).unwrap_err().code, "E104");
    checker
        .required_root(expr.span, |checker| {
            assert_eq!(checker.list_extent(&expr)?, 2);
            Ok(())
        })
        .unwrap();
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn required_extents_share_the_existing_root_and_input_mode() {
    let mut checker = Checker::new();
    let root = Span::new(100, 150);
    checker
        .required_root(root, |checker| {
            assert!(checker.proven_inputs());
            assert_eq!(checker.list_extent(&expression("1+2"))?, 3);
            assert_eq!(checker.list_extent(&expression("{->2}"))?, 2);
            let work = checker.type_work.as_ref().unwrap();
            assert_eq!(work.logical.root, root);
            assert_eq!(work.logical.steps, 6);
            assert_eq!((work.logical.types, work.logical.slots), (0, 0));
            assert!(!work.ordinary);
            Ok(())
        })
        .unwrap();
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn constructor_modes_share_budgets_and_restore_nested_failures() {
    let root = Span::new(100, 150);
    let inner = Span::new(200, 250);
    let mut checker = Checker::new();
    checker
        .mode_root(root, true, |checker| {
            assert!(!checker.proven_inputs());
            checker.list_extent(&expression("1+2"))?;
            for fail in [false, true] {
                let result = checker.mode_root(inner, false, |checker| {
                    assert!(checker.proven_inputs());
                    checker.list_extent(&expression("{->2}"))?;
                    if fail {
                        return Err(Checker::error("E107", "inner failure", inner));
                    }
                    Ok(())
                });
                assert_eq!(result.is_err(), fail);
                assert!(!checker.proven_inputs());
            }
            let budget = &checker.type_work.as_ref().unwrap().logical;
            assert_eq!(budget.root, root);
            assert_eq!(budget.steps, 9);
            Ok(())
        })
        .unwrap();
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn constructor_modes_restore_after_computed_values_and_sticky_limits() {
    let root = Span::new(100, 150);
    let mut checker = Checker::new();
    checker
        .mode_root(root, true, |checker| {
            checker.type_value(&expression("{-><int32[({->2})]>}"))?;
            assert!(!checker.proven_inputs());
            assert_eq!(checker.type_work.as_ref().unwrap().logical.root, root);
            Ok(())
        })
        .unwrap();
    let error = checker
        .mode_root(root, true, |checker| {
            checker.type_work.as_mut().unwrap().logical.steps = MAX_STEPS;
            let error = checker.type_value(&expression("<int32>")).unwrap_err();
            assert_eq!(error.code, "E220");
            assert!(!checker.proven_inputs());
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.span, root);
    assert!(checker.type_work.is_none());
    assert_eq!(checker.list_extent(&expression("2")).unwrap(), 2);
}
