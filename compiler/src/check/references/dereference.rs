use super::*;
use crate::check::dependencies::PointKind;

pub(crate) fn expr(source: &str) -> ast::Expr {
    let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
    let ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    expr
}

pub(crate) fn prefix(checker: &mut Checker, source: &str) {
    for stmt in crate::parser::parse(source).unwrap().stmts {
        checker.stmt(&stmt).unwrap();
    }
}

#[test]
pub(crate) fn deref_roots_preserve_grouped_effectful_pointers_and_exact_results() {
    let mut checker = Checker::new();
    prefix(&mut checker, "n:=1");
    let expr = expr("(({n=2;->&n}))");
    let span = Span::new(100, 120);
    let (outer, (point, value)) = checker
        .with_point_id(PointKind::Expr, span, |checker| {
            checker.deref_point(&expr, span)
        })
        .unwrap();
    assert_eq!(checker.points[point].span, expr.span);
    assert_eq!(checker.points[point].parent, Some(outer));
    assert_eq!(value.span, span);
    let hir::ExprKind::Deref(input) = &value.kind else {
        panic!()
    };
    assert_eq!(input.ty.pointee(), Some(&value.ty));
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| { op.kind == crate::check::dependencies::OperationKind::Write })
            .count(),
        1
    );
    assert!(checker.point.is_none());
    crate::compile("n:=1;x:*(({n=2;->&n}))").unwrap();
}

#[test]
pub(crate) fn deref_roots_preserve_shared_exclusive_aggregate_and_cell_types() {
    for source in [
        "n:1;p:&n",
        "n:=1;p:&!n",
        "row:{->x:1};p:&row",
        "xs:[1];p:&xs",
        "n:1;q:&n;p:&q",
    ] {
        crate::compile(&format!("{source};x:*p")).unwrap();
        let mut checker = Checker::new();
        prefix(&mut checker, source);
        let expr = expr("((p))");
        let (point, value) = checker.deref_point(&expr, expr.span).unwrap();
        let hir::ExprKind::Deref(input) = &value.kind else {
            panic!()
        };
        assert_eq!(input.ty.pointee(), Some(&value.ty));
        assert_eq!(checker.points[point].span, expr.span);
        assert!(checker.points[point].complete);
    }
}

#[test]
pub(crate) fn deref_roots_preserve_stopped_inputs_errors_and_loan_boundaries() {
    let mut checker = Checker::new();
    prefix(&mut checker, r#"d:@"debug""#);
    let expr = expr(r#"d.panic("stop")"#);
    let (point, value) = checker.deref_point(&expr, expr.span).unwrap();
    assert_eq!(value.ty, Type::Never);
    assert!(checker.outputs.contains_key(&point));
    for (source, code) in [("1", "E222"), ("missing", "E201")] {
        let mut checker = Checker::new();
        let expr = self::expr(source);
        checker
            .with_point_id(PointKind::Expr, expr.span, |checker| {
                let prior = checker.point;
                assert_eq!(
                    checker.deref_point(&expr, expr.span).unwrap_err().code,
                    code
                );
                assert_eq!(checker.point, prior);
                Ok(())
            })
            .unwrap();
        assert!(checker.point.is_none());
    }
    for (source, code) in [("n:=1;p:&n;n=2;x:*p", "E302"), ("p:&(1+2);x:*p", "E303")] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
}
