use super::tests::statements;
use crate::ast::Span;
use crate::check::Checker;
use crate::hir::{Expr, ExprKind, Type};
use std::collections::BTreeSet;

pub(crate) fn borrow(checker: &mut Checker, value: Expr) -> Expr {
    checker.statement.push((7, false));
    let result = checker.temporary_borrow(value, Span::new(1, 2)).unwrap();
    assert!(checker.statement.pop().unwrap().1);
    result
}

#[test]
pub(crate) fn temporary_borrows_keep_distinct_statement_owned_origins() {
    let mut checker = Checker::new();
    let value = Expr {
        kind: ExprKind::Bool(false),
        ty: Type::Bool,
        span: Span::new(1, 2),
    };
    let a = borrow(&mut checker, value.clone());
    let b = borrow(&mut checker, value);
    assert_eq!(checker.reference_origins(&a).roots, BTreeSet::from([0]));
    assert_eq!(checker.reference_origins(&b).roots, BTreeSet::from([1]));
    assert!(checker.reference_origins(&a).complete);
    assert_eq!(checker.proofs.temporaries[&0], 7);
    assert_eq!(checker.proofs.temporaries[&1], 7);
    checker.mark_derived(0);
    let id = checker.local(a.ty.clone());
    checker.track_reference(id, &a, false).unwrap();
    assert!(checker.derived_local(id));
}

#[test]
pub(crate) fn temporary_owners_retain_initializer_and_control_dependencies() {
    for controlled in [false, true] {
        let mut checker = Checker::new();
        statements(&mut checker, "seed:false");
        let value = if controlled {
            checker.control = true;
            Expr {
                kind: ExprKind::Bool(false),
                ty: Type::Bool,
                span: Span::new(1, 2),
            }
        } else {
            checker.mark_derived(0);
            Expr {
                kind: ExprKind::Local(0),
                ty: Type::Bool,
                span: Span::new(1, 2),
            }
        };
        let expr = borrow(&mut checker, value);
        let origins = checker.reference_origins(&expr);
        assert_eq!(origins.roots, BTreeSet::from([1]));
        assert!(checker.derived_local(1));
        checker.control = false;
        assert!(checker.derived_expr(&expr));
    }
}

#[test]
pub(crate) fn source_temporary_element_borrows_keep_origins_without_extending_lifetimes() {
    crate::compile("flag:*(&([false,true][1]))").unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, "r:&([false,true][1])");
    let id = checker.locals.len() - 1;
    let origins = &checker.pointees[&id];
    assert!(origins.complete);
    assert_eq!(origins.roots.len(), 1);
    assert!(
        origins
            .roots
            .iter()
            .all(|root| checker.proofs.temporaries.contains_key(root))
    );
    let error = crate::compile("r:&false;flag:*r").unwrap_err().remove(0);
    assert_eq!(error.code, "E303");
}
