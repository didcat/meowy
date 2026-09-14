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
        ("~7", 2),
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
