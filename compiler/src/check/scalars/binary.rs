use super::*;

pub(crate) fn value(ty: Type) -> hir::Expr {
    hir::Expr {
        kind: hir::ExprKind::Local(0),
        ty,
        span: Span::default(),
    }
}

pub(crate) fn record(primary: Type) -> Type {
    Type::Record {
        primary: Box::new(primary),
        fields: vec![hir::Field {
            name: "tag".into(),
            ty: Type::Bool,
            mutable: false,
        }],
    }
}

#[test]
pub(crate) fn binary_plans_capture_actual_primary_projections_without_reclassifying_inner_hir() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let mut checker = Checker::new();
    for (op, a, b, primary) in [
        ("+", record(int.clone()), int.clone(), [true, false]),
        ("==", int.clone(), record(int.clone()), [false, true]),
        (
            "==",
            record(int.clone()),
            record(int.clone()),
            [false, false],
        ),
    ] {
        let (plan, result) = checker
            .binary_plan_values(op, value(a), value(b), Span::default())
            .unwrap();
        assert_eq!(plan.primary, primary);
        assert_eq!(plan.normal, [true, true]);
        let hir::ExprKind::Binary { left, right, .. } = result.kind else {
            panic!()
        };
        assert_eq!(matches!(left.kind, hir::ExprKind::Primary(_)), primary[0]);
        assert_eq!(matches!(right.kind, hir::ExprKind::Primary(_)), primary[1]);
    }
    let (changed, projected) = Checker::projected(value(record(int.clone())));
    assert!(changed);
    let (plan, result) = checker
        .binary_plan_values("+", projected, value(int), Span::default())
        .unwrap();
    assert_eq!(plan.primary, [false, false]);
    let hir::ExprKind::Binary { left, .. } = result.kind else {
        panic!()
    };
    assert!(matches!(left.kind, hir::ExprKind::Primary(_)));
    assert!(checker.points.is_empty());
    assert!(checker.sequences.is_empty());
}

#[test]
pub(crate) fn binary_plans_distinguish_checked_integer_success_and_stopped_projections() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let mut checker = Checker::new();
    for op in [
        "+", "-", "*", "/", "%", "&", "|", "^", "==", "!=", "<", "<=", ">", ">=",
    ] {
        let (plan, _) = checker
            .binary_plan_values(op, value(int.clone()), value(int.clone()), Span::default())
            .unwrap();
        assert_eq!(plan.checked, matches!(op, "+" | "-" | "*" | "/" | "%"));
    }
    for ty in [
        Type::Float { bits: 64 },
        Type::String,
        Type::Reference(Box::new(int.clone())),
    ] {
        let (plan, _) = checker
            .binary_plan_values("==", value(ty.clone()), value(ty), Span::default())
            .unwrap();
        assert!(!plan.checked);
    }
    let (plan, _) = checker
        .binary_plan_values(
            "/",
            value(Type::Float { bits: 64 }),
            value(Type::Float { bits: 64 }),
            Span::default(),
        )
        .unwrap();
    assert!(!plan.checked);
    let (plan, result) = checker
        .binary_plan_values("+", value(record(Type::Never)), value(int), Span::default())
        .unwrap();
    assert_eq!(plan.primary, [true, false]);
    assert_eq!(plan.normal, [false, true]);
    assert!(!plan.checked);
    assert_eq!(result.ty, Type::Never);
}

#[test]
pub(crate) fn binary_plans_preserve_errors_and_required_only_value_construction() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let mut checker = Checker::new();
    assert_eq!(
        checker
            .binary_plan_values("+", value(int.clone()), value(Type::Bool), Span::default())
            .unwrap_err()
            .code,
        "E222"
    );
    let mut a = value(int.clone());
    a.kind = hir::ExprKind::Int(7);
    let mut b = value(int);
    b.kind = hir::ExprKind::Int(0);
    assert_eq!(
        checker
            .binary_plan_values("/", a.clone(), b, Span::default())
            .unwrap_err()
            .code,
        "E107"
    );
    let value = checker
        .binary_values("+", a.clone(), a, Span::default())
        .unwrap();
    assert!(matches!(
        value.kind,
        hir::ExprKind::Binary { point: None, .. }
    ));
    assert!(checker.points.is_empty());
    assert!(checker.sequences.is_empty());
    crate::compile("<T>:{n:1+2;-><uint8[n]>};v<T>:[7]").unwrap();
}
