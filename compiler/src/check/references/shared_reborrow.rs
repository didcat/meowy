use super::dereference::{expr, prefix};
use super::*;
use crate::check::dependencies::PointKind;

#[test]
pub(crate) fn shared_reborrow_roots_preserve_grouped_parents_effects_and_modes() {
    for borrow in ["&", "&!"] {
        let mut checker = Checker::new();
        prefix(&mut checker, "n:=1");
        let expr = expr(&format!("(({{n=2;->{borrow}n}}))"));
        let span = Span::new(100, 120);
        let error = Diagnostic::unsupported("saved address error", span);
        let (outer, (point, value)) = checker
            .with_point_id(PointKind::Expr, span, |checker| {
                checker.shared_reborrow_point(&expr, span, error)
            })
            .unwrap();
        assert_eq!(checker.points[point].parent, Some(outer));
        assert_eq!(checker.points[point].span, expr.span);
        assert_eq!(value.span, span);
        let hir::ExprKind::Reborrow {
            site,
            value: parent,
            fields,
        } = &value.kind
        else {
            panic!()
        };
        assert_eq!(*site, 0);
        assert_eq!(checker.reborrows, 1);
        assert!(fields.is_empty());
        assert_eq!(parent.ty.pointee(), value.ty.pointee());
        assert_eq!(matches!(parent.ty, Type::Exclusive(_)), borrow == "&!");
        assert!(checker.derefs.is_empty());
        assert_eq!(
            checker
                .operations
                .values()
                .filter(|op| { op.kind == crate::check::dependencies::OperationKind::Write })
                .count(),
            1
        );
    }
}

#[test]
pub(crate) fn shared_reborrow_roots_keep_aggregate_and_reference_cell_results() {
    for source in [
        "row:{->n:1;->xs:[2]};p:&row;q:&*p",
        "xs:[1,2];p:&xs;q:&*((p))",
        "n:1;r:&n;p:&r;q:&*p",
        "n:=1;p:&!n;q:&*p;copy:p;x:*q;*copy=2",
    ] {
        crate::compile(source).unwrap();
    }
    for (source, code) in [
        ("n:=1;p:&!n;copy:p;q:&*p", "E301"),
        ("n:=1;p:&!n;q:&*p;*p=2;x:*q", "E302"),
        ("q:&*(&(1+2));x:*q", "E303"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn shared_reborrow_roots_preserve_saved_errors_and_skip_stopped_site_allocation() {
    let mut checker = Checker::new();
    let expr = expr("1");
    let span = Span::new(100, 120);
    let error = Diagnostic::unsupported("saved address error", span);
    checker
        .with_point_id(PointKind::Expr, span, |checker| {
            let prior = checker.point;
            let error = checker
                .shared_reborrow_point(&expr, span, error)
                .unwrap_err();
            assert_eq!(
                error.message,
                "saved address error is not supported by this bootstrap compiler"
            );
            assert_eq!(error.span, span);
            assert_eq!(checker.point, prior);
            Ok(())
        })
        .unwrap();
    assert_eq!(checker.reborrows, 0);
    prefix(&mut checker, r#"d:@"debug""#);
    let expr = self::expr(r#"d.panic("stop")"#);
    let error = Diagnostic::unsupported("saved address error", span);
    let (point, value) = checker.shared_reborrow_point(&expr, span, error).unwrap();
    assert_eq!(value.ty, Type::Never);
    assert!(checker.outputs.contains_key(&point));
    assert_eq!(checker.reborrows, 0);
    assert!(checker.point.is_none());
}
