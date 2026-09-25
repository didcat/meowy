use super::*;

pub(crate) fn expr(source: &str) -> ast::Expr {
    let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
    let ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    expr
}

#[test]
pub(crate) fn format_roots_keep_nested_text_and_primary_values_in_source_order() {
    let source = r#"("a{("b{((1+2))}c")}d{{->7;->extra:8}}e")"#;
    let expr = expr(source);
    let mut checker = Checker::new();
    let mut parts = Vec::new();
    let (id, points) = checker
        .with_point_id(PointKind::Expr, expr.span, |checker| {
            checker.format_parts(&expr, &mut parts)
        })
        .unwrap();
    assert_eq!(points.len(), parts.len());
    let mut texts = Vec::new();
    let mut inputs = Vec::new();
    for (point, value) in points.iter().zip(&parts) {
        if let Some(point) = point {
            let point = &checker.points[point.point];
            assert_eq!(point.parent, Some(id));
            assert!(point.complete);
            inputs.push(&source[point.span.start..point.span.end]);
            assert_eq!(
                value.ty,
                Type::Int {
                    bits: 32,
                    signed: true
                }
            );
        } else {
            let hir::ExprKind::String(text) = &value.kind else {
                panic!()
            };
            texts.push(text.as_str());
        }
    }
    assert_eq!(texts, ["a", "b", "c", "d", "e"]);
    assert_eq!(inputs, ["1+2", "{->7;->extra:8}"]);
    assert_eq!(
        points
            .iter()
            .flatten()
            .map(|input| input.primary)
            .collect::<Vec<_>>(),
        [false, true]
    );
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn format_roots_check_effects_once_and_preserve_nonreturning_operands() {
    let mut checker = Checker::new();
    for stmt in crate::parser::parse("d:@\"debug\";n:=0").unwrap().stmts {
        checker.stmt(&stmt).unwrap();
    }
    let expr = expr(r#""x{{n=1;->n}}{d.panic("stop")}tail""#);
    let mut parts = Vec::new();
    let points = checker.format_parts(&expr, &mut parts).unwrap();
    assert_eq!(points.iter().flatten().count(), 2);
    assert_eq!(
        parts.iter().filter(|part| part.ty == Type::Never).count(),
        1
    );
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| { op.kind == crate::check::dependencies::OperationKind::Write })
            .count(),
        1
    );
}

#[test]
pub(crate) fn format_roots_restore_points_and_keep_original_errors() {
    for (source, code) in [
        ("missing", "E201"),
        ("1+false", "E222"),
        ("[1]", "B001"),
        ("&1", "B001"),
    ] {
        let mut checker = Checker::new();
        let expr = expr(source);
        let mut parts = Vec::new();
        checker
            .with_point_id(PointKind::Expr, expr.span, |checker| {
                let prior = checker.point;
                assert_eq!(
                    checker.format_parts(&expr, &mut parts).unwrap_err().code,
                    code
                );
                assert_eq!(checker.point, prior);
                Ok(())
            })
            .unwrap();
        assert!(parts.is_empty());
        assert!(checker.point.is_none());
    }
}

#[test]
pub(crate) fn format_plans_keep_primary_sources_distinct_from_scalar_results() {
    let mut checker = Checker::new();
    checker
        .stmt(&crate::parser::parse("r:{->7;->tag:true}").unwrap().stmts[0])
        .unwrap();
    let mut parts = Vec::new();
    let points = checker
        .format_parts(&expr(r#""{r}{-r}{r+0}""#), &mut parts)
        .unwrap();
    assert_eq!(
        points
            .iter()
            .flatten()
            .map(|input| input.primary)
            .collect::<Vec<_>>(),
        [true, false, false]
    );
    assert!(matches!(parts[0].kind, hir::ExprKind::Primary(_)));
    assert!(matches!(parts[1].kind, hir::ExprKind::Unary { .. }));
    assert!(matches!(parts[2].kind, hir::ExprKind::Binary { .. }));
}

#[test]
pub(crate) fn format_plans_keep_direct_and_projected_never_with_checked_suffixes() {
    let mut checker = Checker::new();
    for stmt in crate::parser::parse("d:@\"debug\";stop<never>:(){d.panic(\"stop\")}")
        .unwrap()
        .stmts
    {
        checker.stmt(&stmt).unwrap();
    }
    let mut parts = Vec::new();
    let points = checker
        .format_parts(&expr(r#""{stop()}tail{1}""#), &mut parts)
        .unwrap();
    assert!(!points[0].unwrap().primary);
    assert_eq!(parts[0].ty, Type::Never);
    assert!(points[1].is_none());
    assert!(checker.points[points[2].unwrap().point].complete);
    let source = "f<null>:(r<{-><never>;tag<boolean>}>){(@\"debug\").print(r)}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    let output = checker.outputs.values().next().unwrap();
    assert!(output.parts[0].unwrap().primary);
    assert_eq!(output.stopped, Some(0));
}
