use super::super::roots::{expr, setup};
use super::*;

#[test]
pub(crate) fn field_roots_keep_exact_receivers_indices_and_implicit_load_decisions() {
    let mut checker = setup("r:{->z:1;->a:2};p:&r;f<{z<int32>}>:(){->{->z:3}}");
    for (source, name, load) in [
        ("r", "z", false),
        ("((p))", "a", true),
        ("*p", "z", false),
        ("f()", "z", false),
    ] {
        let expr = expr(source);
        let span = Span::new(100, 120);
        let (outer, (input, actual, field)) = checker
            .with_point_id(PointKind::Expr, span, |checker| {
                checker.field_point(&expr, name, span)
            })
            .unwrap();
        assert_eq!(actual, load);
        assert_eq!(checker.points[input].parent, Some(outer));
        assert_eq!(checker.points[input].span, expr.span);
        assert!(checker.points[input].complete);
        assert_eq!(field.span, span);
        let hir::ExprKind::Field { value, index } = field.kind else {
            panic!()
        };
        let Type::Record { fields, .. } = &value.ty else {
            panic!()
        };
        assert_eq!(fields[index].name, name);
        assert_eq!(fields[index].ty, field.ty);
        if load {
            assert!(matches!(value.kind, hir::ExprKind::Deref(_)));
        }
        if source == "*p" {
            assert!(checker.derefs.contains_key(&input));
        }
        if source == "f()" {
            assert!(checker.invocations.values().any(|call| call.point == input));
        }
    }
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn field_roots_keep_declared_union_before_narrowing_and_original_errors() {
    let mut checker = setup("r:{->n<int32><null>:1}");
    let (_, _, field) = checker
        .field_point(&expr("r"), "n", Span::default())
        .unwrap();
    assert!(matches!(field.ty, Type::Union(_)));
    let source = "f<null>:(r<{n<int32><null>}>){|r.n<int32>|{v<int32>:r.n}}";
    crate::compile(source).unwrap();
    for (source, name) in [
        ("missing", "n"),
        ("1", "n"),
        ("r", "missing"),
        (r#"d.panic("stop")"#, "n"),
    ] {
        let mut checker = setup(r#"r:{->n:1};d:@"debug""#);
        let span = Span::new(100, 120);
        checker
            .with_point_id(PointKind::Expr, span, |checker| {
                let prior = checker.point;
                assert_eq!(
                    checker
                        .field_point(&expr(source), name, span)
                        .unwrap_err()
                        .code,
                    "E201"
                );
                assert_eq!(checker.point, prior);
                Ok(())
            })
            .unwrap();
        assert!(checker.point.is_none());
    }
    let mut checker = Checker::new();
    assert!(!checker.flow.spend(usize::MAX));
    let error = checker
        .field_point(&expr("missing"), "n", Span::default())
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation expression budget"));
    assert!(checker.points.is_empty());
}
