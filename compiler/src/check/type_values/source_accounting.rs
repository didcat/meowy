use super::expression_accounting::{cost, expression};
use crate::ast::{ExprKind, Span};
use crate::check::{
    Checker, Result, Spec,
    required::{MAX_STEPS, MAX_TYPES},
};
use crate::hir::Type;

#[test]
pub(crate) fn logical_source_types_count_inputs_before_union_normalization() {
    let mut checker = Checker::new();
    for (source, expected) in [
        ("<int32><int32>", (4, 3)),
        ("(((<int32><int32>)))", (4, 3)),
        ("<never><int32><int32>", (5, 4)),
        ("<int32[2]><int32[2]>", (8, 5)),
        ("<{a<int32><int32>;b<boolean>}>", (7, 6)),
        ("<&int32><&int32>", (6, 5)),
        ("<int32><int32>!<null>", (8, 5)),
    ] {
        assert_eq!(cost(&mut checker, source), expected, "{source}");
    }
}

#[test]
pub(crate) fn logical_source_types_substitute_each_named_payload() {
    let mut checker = Checker::new();
    checker.scopes.last_mut().unwrap().types.insert(
        "Items".into(),
        Spec::Data(Type::List {
            element: Box::new(Type::Bool),
            capacity: 2,
        }),
    );
    assert_eq!(cost(&mut checker, "<Items><Items>"), (6, 5));
    assert_eq!(cost(&mut checker, "<Items><Items>"), (6, 5));
    assert_eq!(cost(&mut checker, "<Items[2]>"), (5, 3));
}

#[test]
pub(crate) fn logical_source_types_keep_failed_prefixes_and_source_order() {
    for (source, code, count) in [
        ("<int32><Missing>", "E202", (3, 2)),
        ("<int32[1/0]><Missing>", "E107", (7, 3)),
        ("<{a<int32>;b<Missing>;c<uint8>}>", "E202", (4, 3)),
        ("<Missing><int32[1/0]>", "E202", (2, 1)),
    ] {
        let expr = expression(source);
        let mut checker = Checker::new();
        let result: Result<()> = checker.required_root(expr.span, |checker| {
            let result = checker.type_value(&expr);
            let work = checker.type_work.as_ref().unwrap();
            assert_eq!((work.logical.steps, work.logical.types), count, "{source}");
            assert_eq!(work.depth, 0);
            result.map(|_| ())
        });
        let error = result.unwrap_err();
        assert_eq!(error.code, code, "{source}");
        if code == "E107" {
            let text = format!("value:{source}");
            assert_eq!(&text[error.span.start..error.span.end], "1/0");
        }
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn logical_source_types_enforce_limits_before_normalizing() {
    let root = Span::new(200, 240);
    for types in [false, true] {
        let total = if types { 3 } else { 4 };
        for remaining in [total - 1, total, total + 1] {
            let expr = expression("<int32><int32>");
            let mut checker = Checker::new();
            let result = checker.required_root(root, |checker| {
                let budget = &mut checker.type_work.as_mut().unwrap().logical;
                if types {
                    budget.types = MAX_TYPES - remaining;
                } else {
                    budget.steps = MAX_STEPS - remaining;
                }
                checker.type_value(&expr)
            });
            assert_eq!(result.is_ok(), remaining >= total);
            if let Err(error) = result {
                assert_eq!(error.code, "E220");
                assert_eq!(error.span, root);
            }
            assert!(checker.type_work.is_none());
            assert_eq!(cost(&mut checker, "<int32><int32>"), (4, 3));
        }
    }
}

#[test]
pub(crate) fn logical_source_types_leave_lookups_and_bootstrap_costs_separate() {
    let expr = expression("<int32><int32>");
    let ExprKind::TypeValue(ty) = &expr.kind else {
        panic!()
    };
    let mut checker = Checker::new();
    checker
        .required_root(expr.span, |checker| {
            checker.spec(ty)?;
            checker.symbol(&expr)?;
            assert!(checker.type_operand_form(&expr, 0, &mut 0)?);
            let work = checker.type_work.as_ref().unwrap();
            assert_eq!(
                (work.logical.steps, work.logical.types, work.nodes),
                (0, 0, 0)
            );
            checker.type_value(&expr)?;
            let work = checker.type_work.as_ref().unwrap();
            assert_eq!(
                (work.logical.steps, work.logical.types, work.nodes),
                (4, 3, 1)
            );
            Ok(())
        })
        .unwrap();
}
