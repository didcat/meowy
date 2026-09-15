use crate::ast::Span;
use crate::diagnostic::Diagnostic;

pub(crate) const MAX_STEPS: usize = 1_000_000;
pub(crate) const MAX_TYPES: usize = 65_536;
pub(crate) const MAX_SLOTS: usize = 1_048_576;

#[derive(Default)]
pub(crate) struct Budget {
    pub(crate) root: Span,
    pub(crate) steps: usize,
    pub(crate) types: usize,
    pub(crate) slots: usize,
    pub(super) failure: Option<Diagnostic>,
}

impl Budget {
    pub(crate) fn charge(&mut self, steps: usize, types: usize) -> Result<(), Diagnostic> {
        self.charge_all(steps, types, 0)
    }

    pub(crate) fn charge_all(
        &mut self,
        steps: usize,
        types: usize,
        slots: usize,
    ) -> Result<(), Diagnostic> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let next = [
            self.steps.checked_add(steps),
            self.types.checked_add(types),
            self.slots.checked_add(slots),
        ];
        for (value, limit, name) in [
            (next[0], MAX_STEPS, "evaluation steps"),
            (next[1], MAX_TYPES, "constructed type nodes"),
            (next[2], MAX_SLOTS, "constructed aggregate slots"),
        ] {
            if value.is_none_or(|value| value > limit) {
                let error = Diagnostic::new(
                    "E220",
                    format!(
                        "required root exceeded {name} limit {limit}; active source-helper stack: empty"
                    ),
                    self.root,
                );
                self.failure = Some(error.clone());
                return Err(error);
            }
        }
        self.steps = next[0].unwrap();
        self.types = next[1].unwrap();
        self.slots = next[2].unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(crate) fn logical_budget_limits_preserve_root_and_first_failure() {
        for (steps, types, name, limit) in [
            (MAX_STEPS, 0, "evaluation steps", MAX_STEPS),
            (0, MAX_TYPES, "constructed type nodes", MAX_TYPES),
        ] {
            let span = Span::new(17, 43);
            let mut budget = Budget {
                root: span,
                ..Budget::default()
            };
            let last = (usize::from(steps != 0), usize::from(types != 0));
            budget.charge(steps - last.0, types - last.1).unwrap();
            budget.charge(last.0, last.1).unwrap();
            let before = (budget.steps, budget.types);
            let error = budget.charge(last.0, last.1).unwrap_err();
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, span);
            assert!(error.message.contains(name));
            assert!(error.message.contains(&limit.to_string()));
            assert_eq!((budget.steps, budget.types), before);
            assert_eq!(budget.charge(0, 0).unwrap_err().message, error.message);
        }
    }

    #[test]
    pub(crate) fn logical_budget_charges_are_atomic_and_overflow_safe() {
        for (steps, types) in [(usize::MAX, 0), (0, usize::MAX)] {
            let mut budget = Budget::default();
            budget.charge(1, 1).unwrap();
            assert_eq!(budget.charge(steps, types).unwrap_err().code, "E220");
            assert_eq!((budget.steps, budget.types), (1, 1));
        }
        let mut repeated = Budget::default();
        for _ in 0..8 {
            repeated.charge(3, 2).unwrap();
        }
        let mut batched = Budget::default();
        batched.charge(24, 16).unwrap();
        assert_eq!(
            (repeated.steps, repeated.types),
            (batched.steps, batched.types)
        );
    }

    #[test]
    pub(crate) fn logical_slots_enforce_exact_limits_without_spending_steps() {
        let span = Span::new(10, 20);
        let mut budget = Budget {
            root: span,
            ..Budget::default()
        };
        budget.charge_all(2, 3, MAX_SLOTS - 1).unwrap();
        budget.charge_all(0, 0, 1).unwrap();
        assert_eq!(
            (budget.steps, budget.types, budget.slots),
            (2, 3, MAX_SLOTS)
        );
        let error = budget.charge_all(1, 1, 1).unwrap_err();
        assert_eq!(error.code, "E220");
        assert_eq!(error.span, span);
        assert!(
            error
                .message
                .contains("constructed aggregate slots limit 1048576")
        );
        assert_eq!(
            (budget.steps, budget.types, budget.slots),
            (2, 3, MAX_SLOTS)
        );
        assert_eq!(budget.charge(1, 0).unwrap_err().message, error.message);
        assert_eq!(
            budget.charge_all(0, 0, 0).unwrap_err().message,
            error.message
        );
    }

    #[test]
    pub(crate) fn logical_slots_reject_overflow_and_keep_combined_charges_atomic() {
        for (steps, types, slots) in [(0, 0, usize::MAX), (MAX_STEPS, 0, 1), (0, MAX_TYPES, 1)] {
            let mut budget = Budget::default();
            budget.charge_all(1, 1, 1).unwrap();
            assert_eq!(
                budget.charge_all(steps, types, slots).unwrap_err().code,
                "E220"
            );
            assert_eq!((budget.steps, budget.types, budget.slots), (1, 1, 1));
        }
        let mut budget = Budget::default();
        for _ in 0..8 {
            budget.charge_all(0, 0, 3).unwrap();
        }
        assert_eq!((budget.steps, budget.types, budget.slots), (0, 0, 24));
    }

    #[test]
    pub(crate) fn logical_slots_share_roots_and_clear_sticky_failures() {
        use crate::check::{Checker, Result};
        let mut checker = Checker::new();
        let root = Span::new(10, 40);
        let inner = Span::new(20, 30);
        let error = checker
            .required_root(root, |checker| {
                checker
                    .type_work
                    .as_mut()
                    .unwrap()
                    .logical
                    .charge_all(0, 0, MAX_SLOTS - 1)?;
                checker.required_root(inner, |checker| {
                    let budget = &mut checker.type_work.as_mut().unwrap().logical;
                    assert_eq!(budget.root, root);
                    budget.charge_all(0, 0, 1)
                })?;
                assert_eq!(checker.type_work.as_ref().unwrap().logical.slots, MAX_SLOTS);
                checker
                    .type_work
                    .as_mut()
                    .unwrap()
                    .logical
                    .charge_all(0, 0, 1)
                    .unwrap_err();
                let result: Result<()> =
                    checker.required_root(inner, |_| panic!("failed root ran"));
                assert_eq!(result.unwrap_err().code, "E220");
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error.span, root);
        assert!(checker.type_work.is_none());
        checker
            .required_root(inner, |checker| {
                let budget = &mut checker.type_work.as_mut().unwrap().logical;
                assert_eq!(budget.slots, 0);
                budget.charge_all(0, 0, 1)
            })
            .unwrap();
        assert!(checker.type_work.is_none());
    }
}
