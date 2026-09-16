use crate::ast::{Expr, Span, StmtKind};
use crate::check::{Checker, Result, Value, required::MAX_STEPS};
use crate::hir::Type;

pub(crate) fn expression(source: &str) -> Expr {
    let block = crate::parser::parse(&format!("value:{source}")).unwrap();
    let StmtKind::Bind { value, .. } = &block.stmts[0].kind else {
        panic!()
    };
    value.clone()
}

pub(crate) fn cost(checker: &mut Checker, source: &str) -> (usize, usize) {
    let expr = expression(source);
    checker
        .required_root(expr.span, |checker| {
            checker.type_value(&expr)?;
            let budget = &checker.type_work.as_ref().unwrap().logical;
            Ok((budget.steps, budget.types))
        })
        .unwrap()
}

#[test]
pub(crate) fn logical_type_expressions_charge_dispatch_and_ignore_groups() {
    let mut checker = Checker::new();
    for (source, expected) in [
        ("<int32>", (2, 1)),
        ("(((<int32>)))", (2, 1)),
        ("1<>", (2, 1)),
        ("<int32>!<null>", (6, 3)),
        ("(((<int32>))!<null>)", (6, 3)),
        ("{-><int32>}", (5, 2)),
        ("((({-><int32>})))", (5, 2)),
        ("({-><int32>})!<null>", (9, 4)),
        ("((({-><int32>})))!<null>", (9, 4)),
    ] {
        assert_eq!(cost(&mut checker, source), expected, "{source}");
    }
}

#[test]
pub(crate) fn logical_type_reads_charge_repeated_values_and_member_ancestors() {
    let mut checker = Checker::new();
    checker
        .declare("kind", Value::Type(Type::Bool), Span::new(0, 4))
        .unwrap();
    assert_eq!(cost(&mut checker, "kind"), (2, 1));
    assert_eq!(cost(&mut checker, "(((kind)))"), (2, 1));
    assert_eq!(cost(&mut checker, "kind!<(kind)>"), (8, 4));
    checker
        .declare(
            "core",
            Value::Module(crate::foundation::Module::Core),
            Span::new(0, 4),
        )
        .unwrap();
    assert_eq!(cost(&mut checker, "core.int32"), (3, 1));
    assert_eq!(cost(&mut checker, "((core)).int32"), (3, 1));
}

#[test]
pub(crate) fn logical_type_expression_limits_preserve_roots_and_depth() {
    let root = Span::new(100, 140);
    for source in ["<int32>!<null>", "(((<int32>))!<null>)"] {
        for remaining in [5, 6, 7] {
            let expr = expression(source);
            let mut checker = Checker::new();
            let scopes = checker.scopes.len();
            let result = checker.required_root(root, |checker| {
                checker.type_work.as_mut().unwrap().logical.steps = MAX_STEPS - remaining;
                let result = checker.type_value(&expr);
                assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
                result
            });
            assert_eq!(result.is_ok(), remaining >= 6, "{source}");
            if let Err(error) = result {
                assert_eq!(error.code, "E220");
                assert_eq!(error.span, root);
            }
            assert_eq!(checker.scopes.len(), scopes);
            assert!(checker.type_work.is_none());
            assert_eq!(cost(&mut checker, source), (6, 3));
        }
    }
}

#[test]
pub(crate) fn logical_type_groups_keep_bootstrap_materialization_guards() {
    let expr = expression("(((<int32>)))");
    let mut checker = Checker::new();
    checker
        .required_root(expr.span, |checker| {
            checker.type_value(&expr)?;
            let work = checker.type_work.as_ref().unwrap();
            assert_eq!((work.visits, work.nodes), (4, 4));
            assert_eq!((work.logical.steps, work.logical.types), (2, 1));
            Ok(())
        })
        .unwrap();
    let error = checker
        .required_root(expr.span, |checker| {
            checker.type_work.as_mut().unwrap().nodes = super::MAX_NODES - 3;
            checker.type_value(&expr)
        })
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn logical_type_form_checks_do_not_evaluate_query_operands() {
    let expr = expression("(((n/0)))<>");
    let mut checker = Checker::new();
    checker
        .declare(
            "n",
            Value::Static {
                value: crate::check::Constant::Int(7),
                ty: Type::Int {
                    bits: 32,
                    signed: true,
                },
            },
            Span::new(0, 1),
        )
        .unwrap();
    let result: Result<()> = checker.required_root(expr.span, |checker| {
        assert!(checker.type_operand_form(&expr, 0, &mut 0)?);
        let budget = &checker.type_work.as_ref().unwrap().logical;
        assert_eq!((budget.steps, budget.types), (0, 0));
        checker.type_value(&expr)?;
        let budget = &checker.type_work.as_ref().unwrap().logical;
        assert_eq!((budget.steps, budget.types), (2, 1));
        Ok(())
    });
    result.unwrap();
}
