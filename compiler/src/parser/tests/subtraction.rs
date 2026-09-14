use super::*;

#[test]
pub(crate) fn type_subtraction_parses_queries_chains_and_preserves_spans() {
    let source = "#é🙂#\nx<(value<>!<null>!<boolean>)>:7";
    let parsed = parse_documented_at(source, 100).unwrap();
    let StmtKind::Bind { ty: Some(ty), .. } = &parsed.block.stmts[0].kind else {
        panic!("binding")
    };
    let TypeKind::Computed(expr) = &ty.kind else {
        panic!("computed")
    };
    let ExprKind::Binary { op, left, right } = &expr.kind else {
        panic!("subtraction")
    };
    assert_eq!(op, "!");
    assert_eq!(expr.span.start, 100 + source.find("value").unwrap());
    assert_eq!(expr.span.end, 100 + source.find(")>").unwrap());
    assert_eq!(right.span.start, 100 + source.find("<boolean>").unwrap());
    assert!(matches!(&left.kind, ExprKind::Binary { op, .. } if op == "!"));
    let ExprKind::Binary { left, .. } = &left.kind else {
        unreachable!()
    };
    assert!(matches!(left.kind, ExprKind::TypeQuery(_)));
    for source in ["x:kind!<null>", "x:kind ! <null>", "x:kind!\n<null>"] {
        assert!(matches!(value(source).kind, ExprKind::Binary { op, .. } if op == "!"));
    }
}

#[test]
pub(crate) fn type_subtraction_parses_annotations_and_retains_other_bang_forms() {
    let block = parse("v<int32><null>!<null>:7").unwrap();
    let StmtKind::Bind { ty: Some(ty), .. } = &block.stmts[0].kind else {
        panic!("binding")
    };
    let TypeKind::Computed(expr) = &ty.kind else {
        panic!("computed")
    };
    let ExprKind::Binary { op, left, right } = &expr.kind else {
        panic!("subtract")
    };
    assert_eq!(op, "!");
    assert!(
        matches!(&left.kind, ExprKind::TypeValue(TypeExpr { kind: TypeKind::Union(parts), .. }) if parts.len() == 2)
    );
    assert!(matches!(right.kind, ExprKind::TypeValue(_)));
    assert!(matches!(value("x:!flag").kind, ExprKind::Unary { op, .. } if op == "!"));
    assert!(matches!(value("x:a!=b").kind, ExprKind::Binary { op, .. } if op == "!="));
    assert!(
        matches!(value("x:kind!<null> == other").kind, ExprKind::Binary { op, .. } if op == "==")
    );
    assert!(
        parse("x:!{}")
            .unwrap_err()
            .iter()
            .any(|error| error.code == "B001")
    );
}

#[test]
pub(crate) fn type_subtraction_bounds_annotation_and_expression_chains() {
    for count in [64, 65] {
        let source = format!("v<int32>{}:7", "!<null>".repeat(count));
        let result = parse(&source);
        if count == 64 {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(result.unwrap_err().iter().any(|error| error.code == "B001"));
        }
    }
    let source = format!("x:kind{}", "!<null>".repeat(260));
    assert!(
        parse(&source)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "B001")
    );
    for source in ["v<int32>!<null><boolean>:7", "x:kind!<null><boolean>"] {
        assert!(
            parse(source)
                .unwrap_err()
                .iter()
                .any(|error| error.code == "B001")
        );
    }
    assert_eq!(
        crate::compile("<T>:<int32><null>!<null>").unwrap_err()[0].code,
        "B001"
    );
}
