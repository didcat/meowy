use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn unary_stages_keep_scalar_types_and_checked_integer_negation() {
    for (source, kind, ty, checked) in [
        (
            "n<int8>:=7;x<int8><null>:-n",
            Kind::Negate,
            hir::Type::Int {
                bits: 8,
                signed: true,
            },
            true,
        ),
        (
            "n<float32>:1.25;x:-n",
            Kind::Negate,
            hir::Type::Float { bits: 32 },
            false,
        ),
        ("x:!(false&&true)", Kind::Not, hir::Type::Bool, false),
        (
            r#"b:@"bits";inv:b.not;n<uint8>:7;x:n.(inv)"#,
            Kind::BitsNot,
            hir::Type::Int {
                bits: 8,
                signed: false,
            },
            false,
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, unary) = checker.unaries.first_key_value().unwrap();
        assert_eq!(unary.kind, kind);
        assert_eq!(unary.ty, ty);
        assert_eq!(unary.span, checker.points[id].span);
        assert_eq!(checker.points[unary.input].parent, Some(id));
        assert_eq!(
            unary.edges,
            [
                Edge::new(Port::Entry(id), Port::Entry(unary.input), Route::Next),
                Edge::new(Port::Normal(unary.input), Port::Operation(id), Route::Next),
                Edge::new(
                    Port::Operation(id),
                    Port::Normal(id),
                    if checked { Route::Checked } else { Route::Next }
                ),
            ]
        );
    }
    assert!(check("x<int8>:-128").0.unaries.is_empty());
}

#[test]
pub(crate) fn unary_stages_preserve_stopped_children_owners_and_control() {
    let source = r#"d:@"debug";stop<never>:(){d.panic("stop")};f<never>:(){!stop()};-f()"#;
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    assert_eq!(checker.unaries.len(), 2);
    for (&id, unary) in &checker.unaries {
        assert_eq!(unary.kind, Kind::Stopped);
        assert_eq!(
            unary.edges,
            [Edge::new(
                Port::Entry(id),
                Port::Entry(unary.input),
                Route::Next
            )]
        );
        assert_eq!(unary.span, checker.points[id].span);
    }
    assert!(checker.unaries.values().any(|unary| unary.owner != 0));
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;|flag|x:!false").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.unaries.values().next().unwrap().control);
    for (source, code) in [
        ("x:!1", "E222"),
        ("x<int8>:-(128)", "E216"),
        ("n<int8>:-128;x:-n", "E107"),
    ] {
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .block(&crate::parser::parse(source).unwrap(), None, None)
                .unwrap_err()
                .code,
            code
        );
        assert!(checker.unaries.is_empty());
    }
}

#[test]
pub(crate) fn unary_stages_validate_identity_and_publish_with_shared_budgets() {
    let (mut checker, block) = check("x:!false");
    let hir::Stmt::Bind { value, .. } = &block.stmts[0] else {
        panic!()
    };
    let (&id, unary) = checker.unaries.first_key_value().unwrap();
    let unary = unary.clone();
    let count = checker.unary_edges;
    checker
        .unary_operation(id, unary.input, value, unary.span)
        .unwrap();
    assert_eq!(checker.unary_edges, count);
    checker.unaries.clear();
    checker.unary_edges = 0;
    let mut invalid = value.clone();
    invalid.ty = hir::Type::String;
    assert!(
        checker
            .unary_operation(id, unary.input, &invalid, unary.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[unary.input].parent = None;
    assert!(
        checker
            .unary_operation(id, unary.input, value, unary.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[unary.input].parent = Some(id);
    checker.output_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .unary_operation(id, unary.input, value, unary.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.unaries.is_empty());
    assert_eq!(checker.unary_edges, 0);
}
