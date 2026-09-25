use super::super::roots::{expr, setup};
use super::*;

#[test]
pub(crate) fn typed_roots_keep_operand_identity_before_target_construction() {
    let mut checker = setup("f<int32>:(){->7};r:{->n:3};n:=0");
    for source in [
        "((1+2))~<int32>",
        "f()<int32>",
        "f()~<int32>",
        "r.n~<int32>",
        "{n=n+1;->7}<({-><int32>})>",
    ] {
        let expr = expr(source);
        let ExprKind::Ascribe { value, ty, .. } = &expr.kind else {
            panic!()
        };
        let (outer, (input, checked, target)) = checker
            .with_point_id(PointKind::Expr, expr.span, |checker| {
                checker.typed_point(value, ty)
            })
            .unwrap();
        assert_eq!(checker.points[input].parent, Some(outer));
        assert_eq!(checker.points[input].span, value.span);
        assert!(checker.points[input].complete);
        assert_eq!(checked.ty, target);
        if matches!(value.kind, ExprKind::Call { .. }) {
            assert!(checker.invocations.values().any(|call| call.point == input));
        }
        if matches!(value.kind, ExprKind::Field { .. }) {
            assert!(checker.fields.contains_key(&input));
        }
    }
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| op.kind == crate::check::dependencies::OperationKind::Write)
            .count(),
        1
    );
    assert!(checker.point.is_none());
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn typed_roots_keep_stopped_inputs_and_operand_before_target_failures() {
    for (source, code) in [
        ("missing~<Missing>", "E201"),
        ("7~<Missing>", "E202"),
        ("(1/0)~<Missing>", "E107"),
        ("d.panic(\"stop\")~<Missing>", "E202"),
        ("d.panic(\"stop\")<int32[1/0]>", "E107"),
    ] {
        let mut checker = setup(r#"d:@"debug""#);
        let expr = expr(source);
        let ExprKind::Ascribe { value, ty, .. } = &expr.kind else {
            panic!()
        };
        checker
            .with_point_id(PointKind::Expr, expr.span, |checker| {
                let prior = checker.point;
                assert_eq!(
                    checker.typed_point(value, ty).unwrap_err().code,
                    code,
                    "{source}"
                );
                assert_eq!(checker.point, prior);
                Ok(())
            })
            .unwrap();
        assert!(checker.point.is_none());
        assert!(checker.type_work.is_none());
    }
    let mut checker = setup(r#"d:@"debug""#);
    let expr = expr(r#"d.panic("stop")~<int32>"#);
    let ExprKind::Ascribe { value, ty, .. } = &expr.kind else {
        panic!()
    };
    let (input, checked, target) = checker.typed_point(value, ty).unwrap();
    assert_eq!(checked.ty, Type::Never);
    assert!(matches!(target, Type::Int { .. }));
    assert!(checker.outputs.contains_key(&input));
    assert!(!checker.flow.spend(usize::MAX));
    let count = checker.points.len();
    let error = checker.typed_point(value, ty).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation expression budget"));
    assert_eq!(checker.points.len(), count);
}

#[test]
pub(crate) fn typed_roots_preserve_predicate_results_noop_hir_and_ascription_errors() {
    let mut checker = Checker::new();
    let (_, value) = checker.expr_point(&expr("7~<int32>"), None).unwrap();
    assert!(matches!(value.kind, hir::ExprKind::Int(7)));
    let (_, value) = checker.expr_point(&expr("7<boolean>"), None).unwrap();
    assert_eq!(value.ty, Type::Bool);
    assert!(matches!(value.kind, hir::ExprKind::TypeTest { .. }));
    assert_eq!(
        checker
            .expr_point(&expr("7~<boolean>"), None)
            .unwrap_err()
            .code,
        "E208"
    );
    assert!(checker.point.is_none());
}
