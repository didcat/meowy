use crate::ast::{Expr, Span, StmtKind};
use crate::check::{Checker, Result};
use crate::hir::Type;

pub(crate) fn value(source: &str) -> Expr {
    let block = crate::parser::parse(&format!("value:{source}")).unwrap();
    let StmtKind::Bind { value, .. } = block.stmts.into_iter().next().unwrap().kind else {
        panic!()
    };
    value
}

pub(crate) fn cost(source: &str, ty: Option<&Type>) -> usize {
    let expr = value(source);
    let mut checker = Checker::new();
    checker
        .required_root(expr.span, |checker| {
            checker.scalar_input(&expr)?;
            assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 0);
            checker.integer_result(&expr, ty)?;
            Ok(checker.type_work.as_ref().unwrap().logical.steps)
        })
        .unwrap()
}

#[test]
pub(crate) fn logical_integer_nodes_preserve_grouping_and_negative_literal_rules() {
    for (source, steps) in [
        ("7", 1),
        ("(((7)))", 1),
        ("1+2*3", 5),
        ("(1)+(2*3)", 5),
        ("-7", 2),
        ("-(7)", 2),
        ("(@\"bits\").not(7)", 2),
    ] {
        assert_eq!(cost(source, None), steps, "{source}");
    }
    assert_eq!(
        cost(
            "-128",
            Some(&Type::Int {
                bits: 8,
                signed: true
            })
        ),
        2
    );
    assert_eq!(
        crate::compile("<T>:{n<int8>:-(128);-><int32>}").unwrap_err()[0].code,
        "E216"
    );
}

#[test]
pub(crate) fn logical_integer_evaluation_stops_at_the_first_arithmetic_failure() {
    let expr = value("(1/0)+(2/0)");
    let mut checker = Checker::new();
    let error = checker
        .required_root(expr.span, |checker| {
            checker.scalar_input(&expr)?;
            let result = checker.integer_result(&expr, None);
            assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 4);
            result.map(|_| ())
        })
        .unwrap_err();
    assert_eq!(error.code, "E107");
    assert_eq!(error.span, Span::new(7, 10));
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn logical_integer_charges_exclude_runtime_folding_and_eligibility() {
    let expr = value("1+2");
    let mut checker = Checker::new();
    checker
        .required_root(expr.span, |checker| {
            checker.expression(&expr, None)?;
            assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 0);
            checker.scalar_input(&expr)?;
            assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 0);
            checker.integer_result(&expr, None)?;
            assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 3);
            assert!(!checker.required);
            Ok(())
        })
        .unwrap();
    let invalid = value("missing");
    let result: Result<()> = checker.required_root(invalid.span, |checker| {
        let result = checker.scalar_input(&invalid);
        assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 0);
        result
    });
    assert_eq!(result.unwrap_err().code, "E201");
}

#[test]
pub(crate) fn logical_integer_negative_literals_preserve_failure_prefix_charges() {
    for source in ["-7", "-(7)"] {
        let expr = value(source);
        let root = Span::new(100, 140);
        let mut checker = Checker::new();
        let result = checker.required_root(root, |checker| {
            checker.type_work.as_mut().unwrap().logical.steps =
                crate::check::required::MAX_STEPS - 1;
            let result = checker.integer_result(&expr, None);
            assert_eq!(
                checker.type_work.as_ref().unwrap().logical.steps,
                crate::check::required::MAX_STEPS
            );
            result.map(|_| ())
        });
        let error = result.unwrap_err();
        assert_eq!(error.code, "E220");
        assert_eq!(error.span, root);
        assert!(!checker.required);
        assert!(checker.type_work.is_none());
    }
}

pub(crate) fn arithmetic_cost(source: &str) -> usize {
    let expr = value(source);
    let mut checker = Checker::new();
    checker
        .required_root(expr.span, |checker| {
            checker.integer_arithmetic(&expr, None)?;
            Ok(checker.type_work.as_ref().unwrap().logical.steps)
        })
        .unwrap()
}

#[test]
pub(crate) fn logical_integer_blocks_charge_operators_and_delegated_nodes_once() {
    for (source, steps) in [
        ("({->2})+3", 5),
        ("({->2})+({->3})", 7),
        ("-({->2})", 4),
        ("(@\"bits\").not({->2})", 4),
        ("-7+({->2})", 6),
        ("({->2})*({->3}+4)", 9),
    ] {
        assert_eq!(arithmetic_cost(source), steps, "{source}");
    }
    for source in ["1+2", "-7+2", "(@\"bits\").not(2*3)"] {
        assert_eq!(arithmetic_cost(source), cost(source, None), "{source}");
    }
}

#[test]
pub(crate) fn logical_integer_comparisons_respect_short_circuit_and_block_costs() {
    for (source, steps) in [
        ("1+2==3", 5),
        ("true||(1/0==0)", 2),
        ("false&&(1+2==3)", 2),
        ("false||(1+2==3)", 7),
        ("({->1})<({->2})", 7),
        ("true||(({->1/0})==1)", 2),
    ] {
        assert_eq!(
            super::accounting::boolean_cost(source, &mut Checker::new()).unwrap(),
            steps,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn logical_integer_extents_reuse_evaluation_and_block_charges() {
    for (source, extent, steps) in [("1+2", 3, 3), ("({->2})+3", 5, 5)] {
        let expr = value(source);
        let mut checker = Checker::new();
        checker
            .required_root(expr.span, |checker| {
                assert_eq!(checker.list_extent(&expr)?, extent);
                assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, steps);
                Ok(())
            })
            .unwrap();
        assert!(!checker.required);
    }
    let mut checker = Checker::new();
    assert_eq!(checker.list_extent(&value("1+2")).unwrap(), 3);
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn logical_integer_block_limits_restore_scope_mode_and_root() {
    let expr = value("({->2})+({->3})");
    let root = Span::new(100, 150);
    for remaining in [6, 7, 8] {
        let mut checker = Checker::new();
        let scopes = checker.scopes.len();
        let result = checker.required_root(root, |checker| {
            checker.type_work.as_mut().unwrap().logical.steps =
                crate::check::required::MAX_STEPS - remaining;
            let result = checker.integer_arithmetic(&expr, None);
            assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
            result.map(|_| ())
        });
        assert_eq!(result.is_ok(), remaining >= 7);
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, root);
        }
        assert_eq!(checker.scopes.len(), scopes);
        assert!(!checker.required);
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn logical_integer_reads_do_not_copy_retained_bootstrap_work() {
    for work in [7, 100] {
        let mut checker = crate::check::inputs::tests::check("n:{->7;unused:1}");
        let id = *checker.inputs.keys().last().unwrap();
        checker.inputs.get_mut(&id).unwrap().work = work;
        checker
            .declare(
                "n",
                crate::check::Value::Local {
                    id,
                    ty: Type::Int {
                        bits: 32,
                        signed: true,
                    },
                    mutable: false,
                    owner: 0,
                    constant: None,
                },
                Span::new(0, 1),
            )
            .unwrap();
        let expr = value("n+n");
        checker
            .required_root(expr.span, |checker| {
                checker.scalar_input(&expr)?;
                assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 0);
                checker.integer_result(&expr, None)?;
                assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 3);
                Ok(())
            })
            .unwrap();
    }
}

#[test]
pub(crate) fn logical_integer_type_query_operands_remain_unevaluated() {
    let mut costs = Vec::new();
    for result in ["<int32>", "n<>", "(n/0)<>"] {
        let source = format!("kind<Type>:{{n:1+2;->{result}}}");
        let block = crate::parser::parse(&source).unwrap();
        let StmtKind::Bind { value, ty, .. } = &block.stmts[0].kind else {
            panic!()
        };
        let mut checker = Checker::new();
        costs.push(
            checker
                .required_root(value.span, |checker| {
                    checker.meta_binding(value, ty.as_ref().unwrap())?;
                    let budget = &checker.type_work.as_ref().unwrap().logical;
                    Ok((budget.steps, budget.types))
                })
                .unwrap(),
        );
    }
    assert_eq!(costs, vec![(10, 3); 3]);
}
