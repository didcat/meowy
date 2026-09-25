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
    let (outer, (point, value)) = checker
        .with_point_id(PointKind::Expr, expr.span, |checker| {
            checker.unary_point("-", &expr, None, expr.span)
        })
        .unwrap();
    assert_eq!(checker.points[point].span, expr.span);
    assert_eq!(checker.points[point].parent, Some(outer));
    assert!(checker.points[point].complete);
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
        let (point, value) = checker
            .unary_point(op, &expr, Some(&expected), expr.span)
            .unwrap();
        assert_eq!(value.ty, ty);
        assert_eq!(checker.points[point].span, expr.span);
        assert!(checker.point.is_none());
    }
    let mut checker = Checker::new();
    checker
        .stmt(&crate::parser::parse(r#"d:@"debug""#).unwrap().stmts[0])
        .unwrap();
    let expr = expr(r#"d.panic("stop")"#);
    let (point, value) = checker.unary_point("!", &expr, None, expr.span).unwrap();
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
