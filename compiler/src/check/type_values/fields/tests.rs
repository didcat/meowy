use crate::ast::Span;
use crate::check::required::MAX_STEPS;
use crate::check::type_values::integer_accounting::value;
use crate::check::{Checker, Value};

pub(crate) fn checker(module: bool) -> Checker {
    let mut checker =
        crate::check::inputs::tests::check("->row:{->part:{->n<uint8>:7;->flag:true}}");
    let id = checker.module.inputs["row"].id;
    let ty = checker.locals[id].clone();
    let entry = if module {
        Value::FileModule { id, ty }
    } else {
        Value::Local {
            id,
            ty,
            mutable: false,
            owner: 0,
            constant: None,
        }
    };
    checker.declare("row", entry, Span::new(0, 3)).unwrap();
    checker
}

#[test]
pub(crate) fn logical_projection_reads_charge_paths_once_and_ignore_groups() {
    for module in [false, true] {
        for (source, boolean, steps) in [
            ("row.part.n", false, 3),
            ("((row).part).n", false, 3),
            ("row.part.n+row.part.n", false, 7),
            ("row.part.flag", true, 3),
            ("((row).part).flag", true, 3),
            ("row.part.flag==row.part.flag", true, 7),
            ("false&&row.part.flag", true, 2),
            ("true||(row.part.n==0)", true, 2),
        ] {
            let mut checker = checker(module);
            let expr = value(source);
            checker
                .required_root(expr.span, |checker| {
                    if boolean {
                        checker.type_boolean(&expr, None)?;
                    } else {
                        checker.scalar_input(&expr)?;
                        assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 0);
                        checker.integer_result(&expr, None)?;
                    }
                    assert_eq!(
                        checker.type_work.as_ref().unwrap().logical.steps,
                        steps,
                        "{source}"
                    );
                    Ok(())
                })
                .unwrap();
        }
    }
}

#[test]
pub(crate) fn logical_projection_lookup_and_runtime_checks_spend_no_steps() {
    let mut checker = checker(false);
    let expr = value("((row).part).n");
    checker
        .required_root(expr.span, |checker| {
            checker.required_path(&expr)?;
            checker.required_field(&expr)?;
            checker.expression(&expr, None)?;
            assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 0);
            checker.integer_result(&expr, None)?;
            assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, 3);
            Ok(())
        })
        .unwrap();
}

#[test]
pub(crate) fn logical_projection_limits_keep_prefix_charges_and_restore_roots() {
    for boolean in [false, true] {
        for remaining in 0..=3 {
            let mut checker = checker(false);
            let expr = value(if boolean {
                "row.part.flag"
            } else {
                "row.part.n"
            });
            let root = Span::new(100, 150);
            let result = checker.required_root(root, |checker| {
                checker.type_work.as_mut().unwrap().logical.steps = MAX_STEPS - remaining;
                let result = if boolean {
                    checker.required_boolean(&expr).map(|_| ())
                } else {
                    checker.integer_result(&expr, None).map(|_| ())
                };
                assert_eq!(checker.type_work.as_ref().unwrap().logical.steps, MAX_STEPS);
                assert_eq!(checker.type_work.as_ref().unwrap().depth, 0);
                result
            });
            assert_eq!(result.is_ok(), remaining == 3);
            if let Err(error) = result {
                assert_eq!(error.code, "E220");
                assert_eq!(error.span, root);
            }
            assert!(!checker.required);
            assert!(checker.type_work.is_none());
            checker
                .required_root(expr.span, |checker| {
                    if boolean {
                        checker.required_boolean(&expr).map(|_| ())
                    } else {
                        checker.integer_result(&expr, None).map(|_| ())
                    }
                })
                .unwrap();
        }
    }
}
