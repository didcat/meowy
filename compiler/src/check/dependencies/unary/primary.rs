use super::{tests::check, *};

#[test]
pub(crate) fn unary_primary_stages_order_record_projection_before_each_operator() {
    for (source, checked) in [
        ("x:-({->2;->tag:true})", true),
        ("x:-({->1.25;->tag:true})", false),
        ("x:!({->true;->tag:1})", false),
        ("b:@\"bits\";x:b.not({n<uint8>:7;->n;->tag:true})", false),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, op) = checker.unaries.first_key_value().unwrap();
        assert!(op.primary);
        let stage = Port::Projection { point: id, step: 0 };
        assert_eq!(
            op.edges,
            [
                Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next),
                Edge::new(Port::Normal(op.input), stage, Route::Next),
                Edge::new(stage, Port::Operation(id), Route::Next),
                Edge::new(
                    Port::Operation(id),
                    Port::Normal(id),
                    if checked { Route::Checked } else { Route::Next }
                ),
            ]
        );
    }
}

#[test]
pub(crate) fn unary_primary_stages_do_not_repeat_expected_projections_or_source_effects() {
    let source = "r:{->2;->tag:true};x:-r";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let op = checker.unaries.values().next().unwrap();
    assert!(!op.primary);
    assert!(checker.coercions[&op.input].primary);
    assert!(
        !op.edges
            .iter()
            .any(|edge| matches!(edge.to, Port::Projection { .. }))
    );
    let source = "n:=0;x:-({n=1;->2;->tag:true})";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| op.kind == super::super::OperationKind::Write)
            .count(),
        1
    );
    assert!(checker.unaries.values().next().unwrap().primary);
    let source = "flag:false;|flag|x:-({->2;->tag:true});f<null>:(){-({->2;->tag:true})}";
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert!(checker.unaries.values().any(|op| op.primary && op.control));
    assert!(checker.unaries.values().any(|op| op.owner != 0));
}

#[test]
pub(crate) fn unary_primary_stages_preserve_stops_literals_and_original_errors() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};x:!stop()";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let op = checker.unaries.values().next().unwrap();
    assert!(!op.primary);
    assert_eq!(op.kind, Kind::Stopped);
    assert_eq!(op.edges.len(), 1);
    assert!(check("x<int8>:-128").0.unaries.is_empty());
    for (source, code) in [
        ("x:-({->true;->tag:1})", "E222"),
        ("n<int8>:-128;x:-n", "E107"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
    let source = "n<int8>:-128;x:-({->n;->tag:true})";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    assert_eq!(
        checker
            .unaries
            .values()
            .next()
            .unwrap()
            .edges
            .last()
            .unwrap()
            .route,
        Route::Checked
    );
}

#[test]
pub(crate) fn unary_primary_stages_validate_captured_flags_and_atomic_shared_capacity() {
    let (mut checker, block) = check("x:-({->2;->tag:true})");
    let hir::Stmt::Bind { value, .. } = &block.stmts[0] else {
        panic!()
    };
    let (&id, op) = checker.unaries.first_key_value().unwrap();
    let op = op.clone();
    let count = checker.unary_edges;
    checker
        .unary_operation(id, op.input, op.primary, value, op.span)
        .unwrap();
    assert_eq!(checker.unary_edges, count);
    assert!(
        checker
            .unary_operation(id, op.input, false, value, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.unaries.clear();
    checker.unary_edges = 0;
    let mut invalid = value.clone();
    invalid.ty = hir::Type::Never;
    assert!(
        checker
            .unary_operation(id, op.input, true, &invalid, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.coercion_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .unary_operation(id, op.input, true, value, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.unaries.is_empty());
    assert_eq!(checker.unary_edges, 0);
}
