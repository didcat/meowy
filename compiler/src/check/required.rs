use crate::ast::Span;
use crate::diagnostic::Diagnostic;

pub(crate) const MAX_STEPS: usize = 1_000_000;
pub(crate) const MAX_TYPES: usize = 65_536;

#[derive(Default)]
pub(crate) struct Budget {
    pub(crate) root: Span,
    pub(crate) steps: usize,
    pub(crate) types: usize,
    pub(super) failure: Option<Diagnostic>,
}

impl Budget {
    pub(crate) fn charge(&mut self, steps: usize, types: usize) -> Result<(), Diagnostic> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let next = [self.steps.checked_add(steps), self.types.checked_add(types)];
        for (value, limit, name) in [
            (next[0], MAX_STEPS, "evaluation steps"),
            (next[1], MAX_TYPES, "constructed type nodes"),
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
}
