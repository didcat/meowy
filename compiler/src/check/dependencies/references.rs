use crate::check::Checker;
use crate::hir::{Expr, ExprKind, Type};

impl Checker {
    pub(crate) fn reference_root(&self, expr: &Expr) -> Option<usize> {
        if !matches!(
            expr.ty.pointee(),
            Some(Type::Bool | Type::Int { .. } | Type::Float { .. })
        ) {
            return None;
        }
        let mut value = expr;
        loop {
            match &value.kind {
                ExprKind::Borrow(place) | ExprKind::ExclusivePath { place, .. } => {
                    return Some(place.root);
                }
                ExprKind::Local(id) => return self.pointees.get(id).copied(),
                ExprKind::Reborrow { value: inner, .. } | ExprKind::Coerce { value: inner } => {
                    value = inner
                }
                _ => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::dependencies::tests::statements;

    #[test]
    pub(crate) fn scalar_reference_copies_observe_later_owner_marks() {
        let mut checker = Checker::new();
        statements(
            &mut checker,
            "x:=false;r:&x;copy:r;other:=false;plain:&other",
        );
        assert_eq!(checker.pointees[&1], 0);
        assert_eq!(checker.pointees[&2], 0);
        assert!(!checker.derived_local(2));
        checker.mark_derived(0);
        assert!(checker.derived_local(1));
        assert!(checker.derived_local(2));
        assert!(!checker.derived_local(4));
        statements(
            &mut checker,
            "flag:*copy;p:@\"proof\";|flag|r:p.can_copy<uint32>()",
        );
        assert!(checker.queries[0].control);
    }

    #[test]
    pub(crate) fn scalar_reborrows_preserve_whole_owner_identity() {
        for source in [
            "x:=false;r:&!x;copy:&!*r",
            "x:=false;r:&!x;copy<&boolean>:r",
        ] {
            let mut checker = Checker::new();
            statements(&mut checker, source);
            assert_eq!(checker.pointees[&1], 0);
            assert_eq!(checker.pointees[&2], 0);
            checker.mark_derived(0);
            assert!(checker.derived_local(2));
        }
    }

    #[test]
    pub(crate) fn mutable_and_aggregate_reference_origins_are_not_invented() {
        for source in ["x:=false;r:=&x;copy:r", "x:{->n:7};r:&x;copy:r"] {
            let mut checker = Checker::new();
            statements(&mut checker, source);
            assert!(checker.pointees.is_empty());
        }
    }
}
