use super::{Checker, Kind, hir};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Literal {
    Null,
    Bool(bool),
    Int(i128),
}

impl Literal {
    pub(super) fn of(value: &hir::Expr) -> Option<Self> {
        match &value.kind {
            hir::ExprKind::Null => Some(Self::Null),
            hir::ExprKind::Bool(value) => Some(Self::Bool(*value)),
            hir::ExprKind::Int(value) => Some(Self::Int(*value)),
            _ => None,
        }
    }
}

impl Kind {
    pub(crate) fn accepts_literal(self, value: Option<Literal>) -> bool {
        match (self, value) {
            (Self::Null, Some(Literal::Null)) | (Self::Bool, Some(Literal::Bool(_))) => true,
            (Self::Int { bits, signed }, Some(Literal::Int(value)))
                if matches!(bits, 8 | 16 | 32 | 64) =>
            {
                Checker::in_range(value, &hir::Type::Int { bits, signed })
            }
            (Self::Float { bits: 32 | 64 }, None) | (Self::String, None) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::expr;
    use super::*;

    #[test]
    pub(crate) fn scalar_literal_capture_preserves_exact_supported_values_and_opaque_kinds() {
        for (source, ty, expected) in [
            ("null", hir::Type::Null, Some(Literal::Null)),
            ("true", hir::Type::Bool, Some(Literal::Bool(true))),
            (
                "-9223372036854775808",
                hir::Type::Int {
                    bits: 64,
                    signed: true,
                },
                Some(Literal::Int(i64::MIN as i128)),
            ),
            (
                "18446744073709551615",
                hir::Type::Int {
                    bits: 64,
                    signed: false,
                },
                Some(Literal::Int(u64::MAX as i128)),
            ),
            ("1.25", hir::Type::Float { bits: 32 }, None),
            ("\"text\"", hir::Type::String, None),
        ] {
            let mut checker = Checker::new();
            checker.expr_point(&expr(source), Some(&ty)).unwrap();
            let leaf = checker.scalar_leaves.first_key_value().unwrap().1;
            assert_eq!(leaf.value, expected);
            assert!(leaf.kind.accepts_literal(leaf.value));
        }
    }

    #[test]
    pub(crate) fn scalar_literal_capture_rejects_changed_values_and_out_of_range_replay() {
        let mut checker = Checker::new();
        let (id, original) = checker.expr_point(&expr("7"), None).unwrap();
        let prior = checker.scalar_leaves.clone();
        let counts = checker.edge_counts();
        for value in [8, i128::MIN, i128::MAX] {
            let mut changed = original.clone();
            changed.kind = hir::ExprKind::Int(value);
            assert!(
                checker
                    .capture_scalar_leaf(id, &changed)
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(checker.scalar_leaves, prior);
            assert_eq!(checker.edge_counts(), counts);
        }
        checker.capture_scalar_leaf(id, &original).unwrap();
    }
}
