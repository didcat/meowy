use super::*;
use crate::check::dependencies::PointKind;

pub(crate) fn expr(source: &str) -> ast::Expr {
    let ast::StmtKind::Expr(expr) = crate::parser::parse(source).unwrap().stmts.remove(0).kind
    else {
        panic!()
    };
    expr
}

pub(crate) fn prepare(source: &str) -> Checker {
    let mut checker = Checker::new();
    for stmt in crate::parser::parse(source).unwrap().stmts {
        checker.stmt(&stmt).unwrap();
    }
    checker
}

#[test]
pub(crate) fn fallback_roots_capture_forwarding_conversion_and_existing_wrappers() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let union = Type::union(vec![int.clone(), Type::Null]);
    let mut checker = prepare("<Choice>:<int32><null>;n:7;r:{->n:1};f<int32>:(){->1}");
    for (source, expected, kind) in [
        ("n", Some(&int), CoercionKind::Forward),
        ("n", Some(&union), CoercionKind::Convert),
        ("n~<Choice>", Some(&union), CoercionKind::Forward),
        ("f()", Some(&union), CoercionKind::Convert),
        ("r", Some(&Type::Bool), CoercionKind::Forward),
        ("true", Some(&int), CoercionKind::Forward),
        ("false&&true", Some(&Type::Bool), CoercionKind::Forward),
    ] {
        let expr = expr(source);
        let (outer, (input, actual, value)) = checker
            .with_point_id(PointKind::Expr, expr.span, |checker| {
                checker.composed_fallback_point(&expr, expected)
            })
            .unwrap();
        assert_eq!(actual, kind, "{source}");
        assert_eq!(checker.points[input].parent, Some(outer));
        assert_eq!(checker.points[input].span, expr.span);
        assert!(checker.points[input].complete);
        if kind == CoercionKind::Convert || source == "n~<Choice>" {
            assert_eq!(value.ty, union);
            assert!(matches!(value.kind, hir::ExprKind::Coerce { .. }));
        }
        if source == "r" {
            assert!(matches!(value.ty, Type::Record { .. }));
        }
        if source == "true" {
            assert_eq!(value.ty, Type::Bool);
        }
        if source == "f()" {
            assert!(checker.invocations.values().any(|call| call.point == input));
        }
        if source == "false&&true" {
            assert!(checker.branch_edges.contains_key(&input));
        }
    }
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn fallback_roots_capture_stops_before_expected_primary_coercion() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    for expected in [None, Some(&int)] {
        let mut checker = prepare(r#"d:@"debug";stop<never>:(){d.panic("stop")}"#);
        let (input, kind, value) = checker
            .composed_fallback_point(&expr("stop()"), expected)
            .unwrap();
        assert_eq!(kind, CoercionKind::Stopped);
        assert_eq!(value.ty, expected.cloned().unwrap_or(Type::Never));
        let call = checker.invocations.values().next().unwrap();
        assert_eq!(call.point, input);
        assert!(!call.may_return);
        if expected.is_some() {
            let hir::ExprKind::Coerce { value } = value.kind else {
                panic!()
            };
            assert_eq!(value.ty, Type::Never);
        }
    }
}

#[test]
pub(crate) fn fallback_roots_preserve_original_errors_and_budget_restoration() {
    for (source, code) in [("missing", "E201"), ("1/0", "E107")] {
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .composed_fallback_point(&expr(source), None)
                .unwrap_err()
                .code,
            code
        );
        assert!(checker.point.is_none());
    }
    let mut checker = Checker::new();
    assert!(!checker.flow.spend(usize::MAX));
    let error = checker
        .composed_fallback_point(&expr("missing"), None)
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation expression budget"));
    assert!(checker.points.is_empty());
}
