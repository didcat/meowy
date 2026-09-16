use crate::ast::Span;
use crate::check::required::MAX_STEPS;
use crate::check::type_values::integer_accounting::value;
use crate::check::{Checker, Value};

pub(crate) fn checker() -> Checker {
    let mut checker =
        crate::check::inputs::tests::check("->row:{->part:{->n<uint8>:7;->flag:true}}");
    let id = checker.module.inputs["row"].id;
    let ty = checker.locals[id].clone();
    checker
        .declare(
            "row",
            Value::Local {
                id,
                ty,
                mutable: false,
                owner: 0,
                constant: None,
            },
            Span::new(0, 3),
        )
        .unwrap();
    checker
}

#[test]
pub(crate) fn logical_record_reads_charge_paths_without_replaying_initializers() {
    for (source, steps) in [("row", 1), ("row.part", 2), ("(((row)).part)", 2)] {
        for cost in [1, 37] {
            let mut checker = checker();
            let id = checker.module.inputs["row"].id;
            checker.record_inputs.get_mut(&id).unwrap().input.work = cost;
            let expr = value(source);
            checker
                .required_root(expr.span, |checker| {
                    for count in 1..=3 {
                        let (_, input) = checker.required_record(&expr)?;
                        assert_eq!(input.input.work, 0);
                        let work = checker.type_work.as_ref().unwrap();
                        assert_eq!(work.logical.steps, steps * count);
                        assert_eq!(work.logical.types, 0);
                        assert!(work.visits >= cost * count);
                    }
                    assert_eq!(checker.record_inputs[&id].input.work, cost);
                    Ok(())
                })
                .unwrap();
        }
    }
}

#[test]
pub(crate) fn logical_record_copies_charge_reads_and_existing_type_construction() {
    let mut checker = checker();
    let expr = value("row.part");
    checker
        .required_root(expr.span, |checker| {
            checker.required_path(&expr)?;
            checker.required_hint(&expr);
            assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 0);
            let copy = checker.type_record(&expr, None)?;
            let budget = &checker.type_work.as_ref().unwrap().logical;
            assert_eq!((budget.steps, budget.types), (6, 4));
            checker.declare("copy", copy, expr.span)?;
            checker.required_root(Span::new(100, 140), |checker| {
                checker.type_record(&value("copy"), None)?;
                let budget = &checker.type_work.as_ref().unwrap().logical;
                assert_eq!((budget.steps, budget.types), (11, 8));
                assert_eq!(budget.root, expr.span);
                Ok(())
            })
        })
        .unwrap();
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn logical_record_read_limits_preserve_prefixes_and_restore_roots() {
    for source in ["row.part", "(((row)).part)"] {
        for remaining in 0..=2 {
            let mut checker = checker();
            let expr = value(source);
            let root = Span::new(100, 150);
            let result = checker.required_root(root, |checker| {
                checker.type_work.as_mut().unwrap().logical.steps = MAX_STEPS - remaining;
                let result = checker.required_record(&expr);
                let work = checker.type_work.as_ref().unwrap();
                assert_eq!(work.logical.steps, MAX_STEPS);
                assert_eq!(work.depth, 0);
                result
            });
            assert_eq!(result.is_ok(), remaining == 2);
            if let Err(error) = result {
                assert_eq!(error.code, "E220");
                assert_eq!(error.span, root);
            }
            assert!(checker.type_work.is_none());
            checker
                .required_root(expr.span, |checker| {
                    checker.required_record(&expr)?;
                    assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 2);
                    Ok(())
                })
                .unwrap();
        }
    }
}

#[test]
pub(crate) fn logical_record_errors_preserve_ancestors_and_budget_precedence() {
    for remaining in [1, 2] {
        let mut checker = checker();
        let id = checker.module.inputs["row"].id;
        let origin = Span::new(200, 210);
        checker.record_inputs.get_mut(&id).unwrap().input.error =
            Some(Checker::error("E107", "retained ancestor failure", origin));
        let expr = value("row.part");
        let root = Span::new(100, 150);
        let error = checker
            .required_root(root, |checker| {
                checker.type_work.as_mut().unwrap().logical.steps = MAX_STEPS - remaining;
                let result = checker.type_record(&expr, None);
                let work = checker.type_work.as_ref().unwrap();
                assert_eq!(work.logical.steps, MAX_STEPS);
                assert_eq!(work.logical.types, 0);
                assert_eq!(work.depth, 0);
                result
            })
            .err()
            .unwrap();
        assert_eq!(error.code, if remaining == 1 { "E220" } else { "E107" });
        assert_eq!(error.span, if remaining == 1 { root } else { origin });
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn logical_record_queries_and_skipped_copies_do_not_read_inputs() {
    let mut costs = Vec::new();
    for selected in [false, true] {
        let mut checker = checker();
        let expr = value(&format!("{{|{selected}|copy:row.part;-><int32>}}"));
        costs.push(
            checker
                .required_root(expr.span, |checker| {
                    checker.type_value(&expr)?;
                    let budget = &checker.type_work.as_ref().unwrap().logical;
                    Ok((budget.steps, budget.types))
                })
                .unwrap(),
        );
    }
    assert_eq!(costs[1].0 - costs[0].0, 7);
    assert_eq!(costs[1].1 - costs[0].1, 4);
    let mut checker = checker();
    let id = checker.module.inputs["row"].id;
    checker.record_inputs.get_mut(&id).unwrap().input.error = Some(Checker::error(
        "E107",
        "unread initializer",
        Span::new(200, 210),
    ));
    for source in ["row.part<>", "((row).part)<>"] {
        let expr = value(source);
        checker
            .required_root(expr.span, |checker| {
                checker.type_value(&expr)?;
                let budget = &checker.type_work.as_ref().unwrap().logical;
                assert_eq!((budget.steps, budget.types), (5, 4));
                Ok(())
            })
            .unwrap();
    }
    let expr = value("{|false|copy:row.part;-><int32>}");
    checker.type_value(&expr).unwrap();
}
