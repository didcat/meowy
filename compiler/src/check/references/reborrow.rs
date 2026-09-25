use super::dereference::{expr, prefix};
use super::*;
use crate::check::dependencies::PointKind;

#[test]
pub(crate) fn reborrow_roots_keep_grouped_effectful_parents_and_existing_sites() {
    let mut checker = Checker::new();
    prefix(&mut checker, "n:=1");
    let expr = expr("(({n=2;->&!n}))");
    let span = Span::new(100, 120);
    let (outer, (point, value)) = checker
        .with_point_id(PointKind::Expr, span, |checker| {
            checker.exclusive_reborrow_point(&expr, span)
        })
        .unwrap();
    assert_eq!(checker.points[point].span, expr.span);
    assert_eq!(checker.points[point].parent, Some(outer));
    assert_eq!(value.span, span);
    let hir::ExprKind::Reborrow {
        site,
        value: parent,
        fields,
    } = value.kind
    else {
        panic!()
    };
    assert_eq!(site, 0);
    assert_eq!(checker.reborrows, 1);
    assert_eq!(parent.ty, value.ty);
    assert!(fields.is_empty());
    assert!(checker.derefs.is_empty());
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| { op.kind == crate::check::dependencies::OperationKind::Write })
            .count(),
        1
    );
    crate::compile("n:=1;q:&!*(({n=2;->&!n}));*q=3").unwrap();
}

#[test]
pub(crate) fn reborrow_roots_do_not_allocate_sites_for_stopped_or_invalid_parents() {
    for (source, code) in [("p", "E305"), ("1", "E305"), ("missing", "E201")] {
        let mut checker = Checker::new();
        prefix(&mut checker, "n:1;p:&n");
        let expr = expr(source);
        checker
            .with_point_id(PointKind::Expr, expr.span, |checker| {
                let prior = checker.point;
                assert_eq!(
                    checker
                        .exclusive_reborrow_point(&expr, expr.span)
                        .unwrap_err()
                        .code,
                    code
                );
                assert_eq!(checker.point, prior);
                Ok(())
            })
            .unwrap();
        assert_eq!(checker.reborrows, 0);
        assert!(checker.point.is_none());
    }
    let mut checker = Checker::new();
    prefix(&mut checker, r#"d:@"debug""#);
    let expr = expr(r#"d.panic("stop")"#);
    let (point, value) = checker.exclusive_reborrow_point(&expr, expr.span).unwrap();
    assert_eq!(value.ty, Type::Never);
    assert!(checker.outputs.contains_key(&point));
    assert_eq!(checker.reborrows, 0);
}

#[test]
pub(crate) fn reborrow_roots_preserve_moves_suspension_and_lifetime_errors() {
    for (source, code) in [
        ("n:=1;p:&!n;copy:p;q:&!*p", "E301"),
        ("n:=1;p:&!n;q:&!*p;x:*p;y:*q", "E302"),
        ("q:{n:=1;p:&!n;->&!*p}", "E303"),
        ("f<null>:(p<&!string>){q:&!*p}", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    crate::compile("n:=1;p:&!n;q:&!*p;copy:p;*q=2;x:*q;*copy=3").unwrap();
}
