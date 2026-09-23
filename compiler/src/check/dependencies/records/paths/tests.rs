use super::*;
use crate::ast::Span;
use crate::hir::{ExprKind, Field};

pub(super) fn record(name: &str, ty: Type) -> Type {
    Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![Field {
            name: name.into(),
            ty,
            mutable: false,
        }],
    }
}

pub(super) fn value(ty: Type) -> Expr {
    Expr {
        kind: ExprKind::Local(0),
        ty,
        span: Span::new(1, 2),
    }
}

#[test]
pub(crate) fn union_paths_keep_shape_identity_at_each_field_boundary() {
    let left = record("a", Type::Reference(Box::new(Type::Bool)));
    let right = record(
        "b",
        Type::Reference(Box::new(Type::Int {
            bits: 32,
            signed: true,
        })),
    );
    let expr = value(record(
        "inner",
        Type::Union(vec![left.clone(), right.clone(), Type::Null]),
    ));
    let mut checker = Checker::new();
    let paths = checker.record_origin_paths(&expr, false).unwrap();
    assert_eq!(paths.len(), 2);
    for path in &paths {
        assert_eq!(path.fields, [0, 0]);
        assert!(path.supported);
        assert_eq!(path.variants.len(), 1);
        assert_eq!(path.variants[0].0, 1);
    }
    assert!(paths.iter().any(|path| path.variants[0].1 == &left));
    assert!(paths.iter().any(|path| path.variants[0].1 == &right));
    assert!(checker.record_paths(&expr).unwrap().is_empty());
}

#[test]
pub(crate) fn union_paths_keep_unknown_alternatives_and_carrier_types() {
    let carrier = Type::Reference(Box::new(Type::Reference(Box::new(Type::Bool))));
    let known = record("c", carrier.clone());
    let unknown = Type::List {
        element: Box::new(carrier),
        capacity: 2,
    };
    let expr = value(Type::Union(vec![
        known.clone(),
        unknown.clone(),
        Type::Null,
    ]));
    let mut checker = Checker::new();
    let paths = checker.record_origin_paths(&expr, true).unwrap();
    assert_eq!(paths.len(), 2);
    let supported = paths.iter().find(|path| path.supported).unwrap();
    assert_eq!(supported.fields, [0]);
    assert_eq!(supported.variants, [(0, &known)]);
    let unsupported = paths.iter().find(|path| !path.supported).unwrap();
    assert!(unsupported.fields.is_empty());
    assert_eq!(unsupported.variants, [(0, &unknown)]);
    assert!(checker.record_paths_for(&expr, true).unwrap().is_empty());
}

#[test]
pub(crate) fn nullable_paths_preserve_the_positional_adapter() {
    let reference = Type::Reference(Box::new(Type::Bool));
    let expr = value(Type::Union(vec![record("r", reference), Type::Null]));
    let mut checker = Checker::new();
    let paths = checker.record_origin_paths(&expr, false).unwrap();
    assert_eq!(paths.len(), 1);
    assert!(paths[0].supported && paths[0].variants.is_empty());
    assert_eq!(checker.record_paths(&expr).unwrap(), [vec![0]]);
    let expr = value(Type::Null);
    assert!(
        checker
            .record_origin_paths(&expr, false)
            .unwrap()
            .is_empty()
    );
}

#[test]
pub(crate) fn union_path_traversal_keeps_depth_capacity_and_work_bounds() {
    let reference = Type::Reference(Box::new(Type::Bool));
    for count in [MAX_DEPTH, MAX_DEPTH + 1] {
        let mut ty = reference.clone();
        for _ in 0..count {
            ty = record("r", ty);
        }
        let expr = value(ty);
        let result = Checker::new().record_origin_paths(&expr, false);
        assert_eq!(result.is_ok(), count == MAX_DEPTH);
    }
    for count in [MAX_FIELDS / 2, MAX_FIELDS / 2 + 1] {
        let expr = value(Type::Union(
            (0..count)
                .map(|index| record(&format!("r{index}"), reference.clone()))
                .collect(),
        ));
        let result = Checker::new().record_origin_paths(&expr, false);
        assert_eq!(result.is_ok(), count == MAX_FIELDS / 2);
    }
    let expr = value(Type::Union(
        (0..MAX_FIELDS + 1)
            .map(|index| record(&format!("r{index}"), reference.clone()))
            .collect(),
    ));
    let error = Checker::new()
        .record_origin_paths(&expr, false)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert_eq!(error.span, expr.span);
    let mut checker = Checker::new();
    assert!(!checker.flow.spend(usize::MAX));
    let expr = value(record("r", reference));
    assert_eq!(
        checker
            .record_origin_paths(&expr, false)
            .err()
            .unwrap()
            .code,
        "B001"
    );
}

#[test]
pub(crate) fn nested_union_paths_keep_each_selection_and_independent_siblings() {
    let reference = Type::Reference(Box::new(Type::Bool));
    let a = record("a", reference.clone());
    let b = record("b", reference.clone());
    let inner = Type::Union(vec![a.clone(), b.clone()]);
    let left = record("inner", inner);
    let right = record("other", reference.clone());
    let expr = value(Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![
            Field {
                name: "stable".into(),
                ty: reference,
                mutable: false,
            },
            Field {
                name: "variant".into(),
                ty: Type::Union(vec![left.clone(), right.clone()]),
                mutable: false,
            },
        ],
    });
    let mut checker = Checker::new();
    let paths = checker.record_origin_paths(&expr, false).unwrap();
    assert_eq!(paths.len(), 4);
    let nested = paths
        .iter()
        .filter(|path| path.variants.len() == 2)
        .collect::<Vec<_>>();
    assert_eq!(nested.len(), 2);
    for path in nested {
        assert_eq!(path.fields, [1, 0, 0]);
        assert_eq!(path.variants[0], (1, &left));
        assert_eq!(path.variants[1].0, 2);
        assert!(path.variants[1].1 == &a || path.variants[1].1 == &b);
    }
    assert!(
        paths
            .iter()
            .any(|path| path.variants == [(1, &right)] && path.fields == [1, 0])
    );
    assert_eq!(checker.record_paths(&expr).unwrap(), [vec![0]]);
}
