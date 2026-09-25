use super::*;

pub(crate) fn expr(source: &str) -> ast::Expr {
    let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
    let ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    expr
}

#[test]
pub(crate) fn unary_roots_preserve_grouped_primary_operands_without_replaying_effects() {
    let mut checker = Checker::new();
    checker
        .stmt(&crate::parser::parse("n:=0").unwrap().stmts[0])
        .unwrap();
    let expr = expr("(({n=1;->2;->extra:3}))");
    let (outer, (point, primary, value)) = checker
        .with_point_id(PointKind::Expr, expr.span, |checker| {
            checker.unary_point("-", &expr, None, expr.span)
        })
        .unwrap();
    assert_eq!(checker.points[point].span, expr.span);
    assert_eq!(checker.points[point].parent, Some(outer));
    assert!(checker.points[point].complete);
    assert!(primary);
    let hir::ExprKind::Unary { value, .. } = value.kind else {
        panic!()
    };
    assert!(matches!(value.kind, hir::ExprKind::Primary(_)));
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| { op.kind == crate::check::dependencies::OperationKind::Write })
            .count(),
        1
    );
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn unary_roots_keep_scalar_context_before_union_coercion_and_stopped_inputs() {
    for (op, source, ty) in [
        ("-", "((1.25))", Type::Float { bits: 32 }),
        (
            "~",
            "((7))",
            Type::Int {
                bits: 8,
                signed: false,
            },
        ),
        ("!", "((false))", Type::Bool),
    ] {
        let mut checker = Checker::new();
        let expr = expr(source);
        let expected = Type::union([ty.clone(), Type::Null]);
        let (point, primary, value) = checker
            .unary_point(op, &expr, Some(&expected), expr.span)
            .unwrap();
        assert_eq!(value.ty, ty);
        assert!(!primary);
        assert_eq!(checker.points[point].span, expr.span);
        assert!(checker.point.is_none());
    }
    let mut checker = Checker::new();
    checker
        .stmt(&crate::parser::parse(r#"d:@"debug""#).unwrap().stmts[0])
        .unwrap();
    let expr = expr(r#"d.panic("stop")"#);
    let (point, primary, value) = checker.unary_point("!", &expr, None, expr.span).unwrap();
    assert!(!primary);
    assert_eq!(value.ty, Type::Never);
    assert_eq!(checker.outputs.len(), 1);
    assert!(checker.outputs.contains_key(&point));
}

#[test]
pub(crate) fn unary_roots_preserve_errors_and_restore_active_points() {
    let ty = Type::Int {
        bits: 8,
        signed: true,
    };
    for (op, source, code) in [
        ("-", "true", "E222"),
        ("-", "(128)", "E216"),
        ("-", "(-128)", "E107"),
        ("!", "missing", "E201"),
    ] {
        let mut checker = Checker::new();
        let expr = expr(source);
        checker
            .with_point_id(PointKind::Expr, expr.span, |checker| {
                let prior = checker.point;
                assert_eq!(
                    checker
                        .unary_point(op, &expr, Some(&ty), expr.span)
                        .unwrap_err()
                        .code,
                    code
                );
                assert_eq!(checker.point, prior);
                Ok(())
            })
            .unwrap();
        assert!(checker.point.is_none());
    }
}

#[test]
pub(crate) fn unary_plans_distinguish_expected_and_inner_primary_wrappers() {
    let mut checker = Checker::new();
    checker
        .stmt(&crate::parser::parse("r:{->2;->tag:true}").unwrap().stmts[0])
        .unwrap();
    let input = expr("r");
    let (point, primary, value) = checker.unary_point("-", &input, None, input.span).unwrap();
    assert!(!primary);
    assert!(checker.coercions[&point].primary);
    let hir::ExprKind::Unary { value, .. } = value.kind else {
        panic!()
    };
    assert!(matches!(value.kind, hir::ExprKind::Primary(_)));
    let (primary, value) = checker.unary_plan_value("-", *value, input.span).unwrap();
    assert!(!primary);
    assert!(matches!(value.kind, hir::ExprKind::Unary { .. }));
}

#[test]
pub(crate) fn unary_plans_preserve_invalid_primaries_and_source_free_construction() {
    let span = crate::ast::Span::default();
    for primary in [Type::Never, Type::Bool] {
        let value = hir::Expr {
            kind: hir::ExprKind::Local(0),
            span,
            ty: Type::Record {
                primary: Box::new(primary),
                fields: vec![hir::Field {
                    name: "tag".into(),
                    ty: Type::Bool,
                    mutable: false,
                }],
            },
        };
        let mut checker = Checker::new();
        assert_eq!(
            checker.unary_plan_value("-", value, span).unwrap_err().code,
            "E222"
        );
        assert!(checker.points.is_empty());
        assert!(checker.unaries.is_empty());
    }
    let mut checker = Checker::new();
    let value = hir::Expr {
        kind: hir::ExprKind::Int(7),
        ty: Type::Int {
            bits: 32,
            signed: true,
        },
        span,
    };
    let result = checker.unary_value("-", value, span).unwrap();
    assert!(matches!(result.kind, hir::ExprKind::Unary { .. }));
    assert!(checker.points.is_empty());
    assert!(checker.unaries.is_empty());
}
