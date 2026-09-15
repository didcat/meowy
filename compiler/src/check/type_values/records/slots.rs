use crate::check::{Result, type_values::Work};
use crate::hir::Type;

impl Work {
    pub(crate) fn slot(&mut self) -> Result<()> {
        self.logical.charge_all(0, 0, 1)
    }

    pub(crate) fn record_slots(&mut self, ty: &Type) -> Result<()> {
        let Type::Record { fields, .. } = ty else {
            unreachable!()
        };
        self.slot()?;
        for field in fields {
            self.slot()?;
            if matches!(field.ty, Type::Record { .. }) {
                self.record_slots(&field.ty)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::Span;
    use crate::check::required::MAX_SLOTS;
    use crate::check::type_values::integer_accounting::value;
    use crate::check::{Checker, Result};

    pub(crate) fn cost(source: &str) -> usize {
        let expr = value(source);
        let mut checker = Checker::new();
        checker
            .required_root(expr.span, |checker| {
                checker.type_value(&expr)?;
                Ok(checker.type_work.as_ref().unwrap().logical.slots)
            })
            .unwrap()
    }

    #[test]
    pub(crate) fn logical_record_slots_count_nested_construction_and_copies() {
        for (source, slots) in [
            ("{r:{->n:4};-><int32>}", 2),
            ("{r:({->part:{->n:4}});-><int32>}", 4),
            ("{r:{->part:{->n:4}};copy:r;again:copy;-><int32>}", 12),
            ("{r:{->part:{->n:4}};copy:((r).part);-><int32>}", 6),
            ("{r<{part<{n<int32>}>}>:{->part:{->n:4}};-><int32>}", 4),
            ("{r:{->n:4};outer:{->part:r};-><int32>}", 6),
            ("{r:{->n:4};n:r.n;-><int32[n]>}", 2),
            ("{r:{->n:4};->r<>}", 2),
            ("{r:{->n:4};|false|copy:r;-><int32>}", 2),
            ("{|false|r:{->n:4};-><int32[1024]>}", 0),
        ] {
            assert_eq!(cost(source), slots, "{source}");
        }
    }

    #[test]
    pub(crate) fn logical_record_slots_transfer_composition_without_duplicate_charges() {
        for (source, slots) in [
            ("{r:{->{->n:4}};-><int32>}", 2),
            ("{base:{->n:4};r:{->base};-><int32>}", 4),
            (
                "{base:{->part:{->n:4}};r:{->base;->flag:true};-><int32>}",
                9,
            ),
            (
                "{r<{n<uint8>;flag<boolean>}>:{->{->n:4};->flag:true};-><int32>}",
                3,
            ),
            (
                "{base:{->part:{->n:4}};r<{part<{n<int32>}>}>:{->base};-><int32>}",
                8,
            ),
            (
                "{r:{->part:{->n:4};|true|->flag:true;|false|->{->unused:7}};-><int32>}",
                5,
            ),
        ] {
            assert_eq!(cost(source), slots, "{source}");
        }
    }

    #[test]
    pub(crate) fn logical_record_slots_stop_at_initialization_and_restore_scopes() {
        for (source, remaining, code) in [
            ("{r:{->a:4;->b:1/0};-><int32>}", 0, "E220"),
            ("{r:{->a:4;->b:1/0};-><int32>}", 1, "E107"),
            ("{r:{->a:1/0};-><int32>}", 0, "E107"),
            ("{r:{->a:4};-><int32>}", 1, "E220"),
            ("{r:{->a:4;->a:5};-><int32>}", 1, "E205"),
            ("{r<{a<int32>;b<int32>}>:{->a:4};-><int32>}", 1, "E204"),
        ] {
            let expr = value(source);
            let mut checker = Checker::new();
            let scopes = checker.scopes.len();
            let root = Span::new(100, 150);
            let error = checker
                .required_root(root, |checker| {
                    checker.type_work.as_mut().unwrap().logical.slots = MAX_SLOTS - remaining;
                    let result = checker.type_value(&expr);
                    let work = checker.type_work.as_ref().unwrap();
                    assert_eq!(work.logical.slots, MAX_SLOTS);
                    assert_eq!(work.depth, 0);
                    result
                })
                .unwrap_err();
            assert_eq!(error.code, code, "{source}");
            if code == "E220" {
                assert_eq!(error.span, root);
            }
            assert_eq!(checker.scopes.len(), scopes);
            assert!(checker.type_work.is_none());
            assert!(checker.locals.is_empty());
            let result: Result<()> = checker.required_root(expr.span, |checker| {
                assert_eq!(checker.type_work.as_ref().unwrap().logical.slots, 0);
                checker.type_value(&value("{r:{->n:4};-><int32>}"))?;
                Ok(())
            });
            result.unwrap();
        }
    }

    #[test]
    pub(crate) fn logical_record_slots_bound_recursive_copies_and_keep_consumed_prefix() {
        for remaining in 0..=5 {
            let mut checker = super::super::accounting::checker();
            let expr = value("row");
            let root = Span::new(100, 150);
            let result = checker.required_root(root, |checker| {
                checker.type_work.as_mut().unwrap().logical.slots = MAX_SLOTS - remaining;
                let result = checker.type_record(&expr, None);
                assert_eq!(checker.type_work.as_ref().unwrap().logical.slots, MAX_SLOTS);
                result
            });
            assert_eq!(result.is_ok(), remaining == 5);
            if let Err(error) = result {
                assert_eq!(error.code, "E220");
                assert_eq!(error.span, root);
            }
            assert!(checker.type_work.is_none());
        }
    }

    #[test]
    pub(crate) fn logical_record_slots_do_not_allocate_for_reads_or_failed_copies() {
        let mut checker = super::super::accounting::checker();
        let expr = value("row.part");
        checker
            .required_root(expr.span, |checker| {
                checker.required_record(&expr)?;
                checker.type_value(&value("row.part<>"))?;
                checker.integer_result(&value("row.part.n"), None)?;
                checker.required_boolean(&value("row.part.flag"))?;
                assert_eq!(checker.type_work.as_ref().unwrap().logical.slots, 0);
                checker.type_record(&expr, None)?;
                assert_eq!(checker.type_work.as_ref().unwrap().logical.slots, 3);
                Ok(())
            })
            .unwrap();
        let id = checker.module.inputs["row"].id;
        let origin = Span::new(200, 210);
        checker.record_inputs.get_mut(&id).unwrap().input.error =
            Some(Checker::error("E107", "retained ancestor failure", origin));
        let error = checker
            .required_root(expr.span, |checker| {
                let result = checker.type_record(&expr, None);
                assert_eq!(checker.type_work.as_ref().unwrap().logical.slots, 0);
                result
            })
            .err()
            .unwrap();
        assert_eq!(error.code, "E107");
        assert_eq!(error.span, origin);
        assert!(checker.type_work.is_none());
    }

    #[test]
    pub(crate) fn logical_record_slots_share_nested_evaluation_and_reset_independent_roots() {
        let mut checker = Checker::new();
        let expr = value("{r:{->n:4};copy:r;-><int32>}");
        let root = Span::new(100, 150);
        for _ in 0..2 {
            checker
                .required_root(root, |checker| {
                    assert_eq!(checker.type_work.as_ref().unwrap().logical.slots, 0);
                    for count in 1..=3 {
                        checker.type_value(&expr)?;
                        let budget = &checker.type_work.as_ref().unwrap().logical;
                        assert_eq!(budget.root, root);
                        assert_eq!(budget.slots, 4 * count);
                    }
                    Ok(())
                })
                .unwrap();
            assert!(checker.type_work.is_none());
            assert!(checker.locals.is_empty());
        }
    }
}
