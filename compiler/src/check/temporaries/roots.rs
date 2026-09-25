use super::*;
use crate::check::dependencies::PointKind;

pub(crate) fn expr(source: &str) -> ast::Expr {
    let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
    let ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    expr
}

#[test]
pub(crate) fn temporary_roots_keep_initializer_order_and_statement_owned_cells() {
    let mut checker = Checker::new();
    checker
        .stmt(&crate::parser::parse("n:=0").unwrap().stmts[0])
        .unwrap();
    let expr = expr("(({n=1;->7}))");
    let span = Span::new(100, 120);
    checker.statement.push((17, false));
    let (outer, (point, value)) = checker
        .with_point_id(PointKind::Expr, span, |checker| {
            checker.temporary_borrow_point(&expr, span)
        })
        .unwrap();
    assert_eq!(checker.points[point].span, expr.span);
    assert_eq!(checker.points[point].parent, Some(outer));
    assert_eq!(value.span, span);
    let ExprKind::TemporaryBorrow {
        id,
        statement,
        value,
    } = value.kind
    else {
        panic!()
    };
    assert_eq!(statement, 17);
    assert_eq!(checker.proofs.temporaries[&id], statement);
    assert_eq!(checker.locals[id], value.ty);
    assert!(checker.statement.pop().unwrap().1);
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
pub(crate) fn temporary_roots_keep_scalar_aggregate_and_reference_values_distinct() {
    for source in ["7", "{->n:1}", "[1,2]", "&n"] {
        let mut checker = Checker::new();
        checker
            .stmt(&crate::parser::parse("n:1").unwrap().stmts[0])
            .unwrap();
        let expr = expr(source);
        checker.statement.push((23, false));
        let (point, value) = checker.temporary_borrow_point(&expr, expr.span).unwrap();
        let ExprKind::TemporaryBorrow {
            id,
            statement,
            value: input,
        } = &value.kind
        else {
            panic!()
        };
        assert_eq!(*statement, 23);
        assert_eq!(value.ty.pointee(), Some(&input.ty));
        assert_eq!(checker.locals[*id], input.ty);
        assert_eq!(checker.points[point].span, expr.span);
        assert_ne!(*id, 0);
        if source == "&n" {
            assert!(matches!(input.ty, Type::Reference(_)));
        }
        assert!(checker.statement.pop().unwrap().1);
    }
}

#[test]
pub(crate) fn temporary_roots_preserve_stopped_inputs_missing_statements_and_errors() {
    let mut checker = Checker::new();
    checker
        .stmt(&crate::parser::parse(r#"d:@"debug""#).unwrap().stmts[0])
        .unwrap();
    let root = expr(r#"d.panic("stop")"#);
    let (point, value) = checker.temporary_borrow_point(&root, root.span).unwrap();
    assert_eq!(value.ty, Type::Never);
    assert!(checker.outputs.contains_key(&point));
    assert!(checker.proofs.temporaries.is_empty());
    for (source, code) in [("missing", "E201"), ("1", "B001")] {
        let mut checker = Checker::new();
        let root = expr(source);
        checker
            .with_point_id(PointKind::Expr, root.span, |checker| {
                let prior = checker.point;
                assert_eq!(
                    checker
                        .temporary_borrow_point(&root, root.span)
                        .unwrap_err()
                        .code,
                    code
                );
                assert_eq!(checker.point, prior);
                Ok(())
            })
            .unwrap();
        assert!(checker.proofs.temporaries.is_empty());
        assert!(checker.point.is_none());
    }
    assert_eq!(crate::compile("p:&1;x:*p").unwrap_err()[0].code, "E303");
    assert_eq!(crate::compile("n:=1;p:&(&!n)").unwrap_err()[0].code, "B001");
}
