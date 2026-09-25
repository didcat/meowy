use super::*;

pub(crate) fn value(ty: Type) -> hir::Expr {
    hir::Expr {
        kind: hir::ExprKind::Local(0),
        ty,
        span: Span::new(1, 2),
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
pub(crate) fn expected_plans_preserve_identity_conversion_and_primary_selection() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let union = Type::union(vec![int.clone(), Type::Null]);
    for (source, target, primary, kind) in [
        (int.clone(), int.clone(), false, CoercionKind::Forward),
        (int.clone(), union.clone(), false, CoercionKind::Convert),
        (
            record(int.clone()),
            int.clone(),
            true,
            CoercionKind::Forward,
        ),
        (
            record(int.clone()),
            union.clone(),
            true,
            CoercionKind::Convert,
        ),
        (
            record(int.clone()),
            record(int.clone()),
            false,
            CoercionKind::Forward,
        ),
    ] {
        let span = Span::new(10, 20);
        let (projected, actual, result) =
            Checker::expected_plan(value(source), &target, span).unwrap();
        assert_eq!(projected, primary);
        assert_eq!(actual, kind);
        assert_eq!(result.ty, target);
        if primary {
            assert_eq!(result.span, span);
        }
        if kind == CoercionKind::Convert {
            assert!(matches!(result.kind, hir::ExprKind::Coerce { .. }));
        }
    }
    let inner = Checker::coerce(value(int), union.clone());
    let (primary, kind, result) = Checker::expected_plan(inner, &union, Span::default()).unwrap();
    assert!(!primary);
    assert_eq!(kind, CoercionKind::Forward);
    assert!(matches!(result.kind, hir::ExprKind::Coerce { .. }));
}

#[test]
pub(crate) fn expected_plans_keep_never_before_and_after_primary_extraction() {
    for source in [Type::Never, record(Type::Never)] {
        let primary = source != Type::Never;
        let (projected, kind, result) =
            Checker::expected_plan(value(source), &Type::Bool, Span::default()).unwrap();
        assert_eq!(projected, primary);
        assert_eq!(kind, CoercionKind::Stopped);
        assert_eq!(result.ty, if primary { Type::Bool } else { Type::Never });
        if primary {
            let hir::ExprKind::Coerce { value } = result.kind else {
                panic!()
            };
            assert_eq!(value.ty, Type::Never);
            assert!(matches!(value.kind, hir::ExprKind::Primary(_)));
        }
    }
}

#[test]
pub(crate) fn expected_plans_preserve_errors_spans_and_source_free_api() {
    let span = Span::new(10, 20);
    for source in [Type::Bool, record(Type::Bool)] {
        let error = Checker::expected_plan(value(source), &Type::String, span).unwrap_err();
        assert_eq!(error.code, "E207");
        assert_eq!(error.span, span);
    }
    let result = Checker::expected_value(value(record(Type::Bool)), &Type::Bool, span).unwrap();
    assert_eq!(result.ty, Type::Bool);
    assert!(matches!(result.kind, hir::ExprKind::Primary(_)));
    crate::compile("<T>:{n<int32>:1+2;-><uint8[n]>};v<T>:[7]").unwrap();
}
