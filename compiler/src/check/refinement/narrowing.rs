use super::*;

pub(crate) fn local(ty: Type) -> hir::Expr {
    hir::Expr {
        kind: hir::ExprKind::Local(0),
        ty,
        span: Span::new(1, 2),
    }
}

#[test]
pub(crate) fn narrowing_plans_keep_changes_distinct_from_existing_wrappers() {
    let ty = Type::union(vec![Type::Bool, Type::Null]);
    let mut checker = Checker::new();
    checker.tags.insert(
        ((0, Vec::new()), ty.clone()),
        vec![(Type::Bool, TRUE), (Type::Null, FALSE)],
    );
    let (changed, value) = checker.narrow_plan(local(ty.clone())).unwrap();
    assert!(changed);
    assert_eq!(value.ty, Type::Bool);
    assert!(matches!(value.kind, hir::ExprKind::Coerce { .. }));
    assert_eq!(value.span, Span::new(1, 2));
    let (changed, value) = checker.narrow_plan(value).unwrap();
    assert!(!changed);
    assert!(matches!(value.kind, hir::ExprKind::Coerce { .. }));
    checker.reach = FALSE;
    let (changed, value) = checker.narrow_plan(local(ty.clone())).unwrap();
    assert!(!changed);
    assert_eq!(value.ty, ty);
    let value = hir::Expr {
        kind: hir::ExprKind::Bool(true),
        ty: Type::Bool,
        span: Span::default(),
    };
    assert!(!checker.narrow_plan(value).unwrap().0);
}

#[test]
pub(crate) fn narrowing_plans_preserve_never_and_mutable_observations() {
    let ty = Type::union(vec![Type::Bool, Type::Null]);
    let mut checker = Checker::new();
    checker.proofs.mutable.insert(0);
    let tags = vec![(Type::Bool, TRUE), (Type::Null, FALSE)];
    checker
        .tags
        .insert(((0, Vec::new()), ty.clone()), tags.clone());
    let (changed, value) = checker.narrow_plan(local(ty.clone())).unwrap();
    assert!(changed);
    assert_eq!(value.ty, Type::Bool);
    assert_eq!(checker.proofs.observations[&(1, 2)], tags);
    checker.tags.insert(
        ((0, Vec::new()), ty.clone()),
        vec![(Type::Bool, FALSE), (Type::Null, FALSE)],
    );
    let (changed, value) = checker.narrow_plan(local(ty)).unwrap();
    assert!(changed);
    assert_eq!(value.ty, Type::Never);
    let (changed, value) = checker.narrow_plan(local(Type::Never)).unwrap();
    assert!(!changed);
    assert_eq!(value.ty, Type::Never);
}

#[test]
pub(crate) fn narrowing_plans_retain_field_paths_and_existing_type_errors() {
    let ty = Type::union(vec![Type::Bool, Type::Null]);
    let record = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![hir::Field {
            name: "n".into(),
            ty: ty.clone(),
            mutable: false,
        }],
    };
    let mut checker = Checker::new();
    checker.tags.insert(
        ((0, vec!["n".into()]), ty.clone()),
        vec![(Type::Bool, TRUE), (Type::Null, FALSE)],
    );
    let value = hir::Expr {
        kind: hir::ExprKind::Field {
            value: Box::new(local(record)),
            index: 0,
        },
        ty,
        span: Span::new(3, 6),
    };
    let (changed, value) = checker.narrow_plan(value).unwrap();
    assert!(changed);
    assert_eq!(value.ty, Type::Bool);
    assert_eq!(Checker::place(&value), Some((0, vec!["n".into()])));
    for (source, code) in [
        ("f:(v<int32><null>){x<int32>:v}", "E207"),
        ("f:(v<int32><null>){x:v~<int32>}", "E208"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
    crate::compile("f:(v<int32><null>){|v<int32>|x<int32>:v}").unwrap();
}
