use super::super::roots::{child, expr, setup};
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

#[test]
pub(crate) fn expected_roots_separate_calls_binary_and_branch_sources_from_caller_results() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let union = Type::union(vec![int.clone(), Type::Null]);
    let mut checker = setup("f<int32>:(){->1};r:{->1;->tag:true}");
    for (source, expected) in [
        ("f()", &union),
        ("1+2", &int),
        ("false&&true", &Type::Bool),
        ("r", &int),
    ] {
        let expr = expr(source);
        let (root, value) = checker.expr_point(&expr, Some(expected)).unwrap();
        let raw = child(&checker, root);
        assert_eq!(checker.points[root].kind, PointKind::Expr);
        assert_eq!(checker.points[raw].span, expr.span);
        assert_eq!(value.ty, *expected);
        match source {
            "f()" => assert!(checker.invocations.values().any(|call| call.point == raw)),
            "1+2" => assert!(checker.binaries.contains_key(&raw)),
            "false&&true" => {
                assert_eq!(checker.points[raw].kind, PointKind::And);
                assert!(checker.branch_edges.contains_key(&raw));
            }
            "r" => assert!(matches!(value.kind, hir::ExprKind::Primary(_))),
            _ => unreachable!(),
        }
    }
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn expected_roots_preserve_required_paths_shared_sites_and_stopped_types() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let mut checker = Checker::new();
    checker.required = true;
    let (root, _) = checker.expr_point(&expr("1+2"), Some(&int)).unwrap();
    assert!(
        checker
            .sequences
            .contains_key(&crate::check::dependencies::SequenceSource::Expr(root))
    );
    assert!(checker.binaries.is_empty());
    let mut checker = setup("n:=1;p:&!n");
    let shared = Type::Reference(Box::new(int.clone()));
    let (root, _) = checker.expr_point(&expr("p"), Some(&shared)).unwrap();
    assert_eq!(checker.reborrow_ops[&root].parent, child(&checker, root));
    assert_eq!(checker.reborrows, 1);
    let mut checker = setup("d:@\"debug\"");
    let (root, value) = checker
        .expr_point(&expr("d.panic(\"stop\")"), Some(&int))
        .unwrap();
    assert_eq!(value.ty, Type::Never);
    assert!(checker.outputs.contains_key(&child(&checker, root)));
    assert!(checker.reborrow_ops.is_empty());
    assert_eq!(checker.reborrows, 0);
}

#[test]
pub(crate) fn expected_roots_preserve_errors_and_budget_precedence() {
    for source in ["missing", "1"] {
        let mut checker = Checker::new();
        let error = checker
            .expr_point(&expr(source), Some(&Type::Bool))
            .unwrap_err();
        assert_eq!(
            error.code,
            if source == "missing" { "E201" } else { "E207" }
        );
        assert!(checker.point.is_none());
        assert!(!checker.points[0].complete);
    }
    let mut checker = Checker::new();
    assert!(!checker.flow.spend(usize::MAX));
    let error = checker
        .expr_point(&expr("missing"), Some(&Type::Bool))
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation expression budget"));
    assert!(checker.points.is_empty());
}
