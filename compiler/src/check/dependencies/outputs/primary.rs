use super::{tests::check, *};

#[test]
pub(crate) fn output_primary_stages_preserve_interleaved_text_projection_and_io_order() {
    for method in ["print", "panic"] {
        let source = format!(r#"d:@"debug";d.{method}("a{{({{->7;->tag:true}})}}b{{1}}")"#);
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let (&id, op) = checker.outputs.first_key_value().unwrap();
        let input = op.parts[1].unwrap();
        assert!(input.primary);
        assert!(!op.parts[3].unwrap().primary);
        assert!(op.parts[0].is_none() && op.parts[2].is_none());
        let projection = Port::Projection { point: id, step: 1 };
        for edge in [
            Edge::new(
                Port::Output { point: id, part: 0 },
                Port::Entry(input.point),
                Route::Returned,
            ),
            Edge::new(Port::Normal(input.point), projection, Route::Next),
            Edge::new(projection, Port::Output { point: id, part: 1 }, Route::Next),
            Edge::new(
                Port::Output { point: id, part: 1 },
                Port::Output { point: id, part: 2 },
                Route::Returned,
            ),
        ] {
            assert!(op.edges.contains(&edge));
        }
        assert!(!op.edges.contains(&Edge::new(
            Port::Normal(input.point),
            Port::Output { point: id, part: 1 },
            Route::Next
        )));
        if method == "panic" {
            assert_eq!(
                op.edges[0],
                Edge::new(Port::Entry(id), Port::Prefix(id), Route::Next)
            );
            assert!(!op.edges.iter().any(|edge| edge.to == Port::Normal(id)));
        }
    }
}

#[test]
pub(crate) fn output_primary_stages_stop_after_never_projection_without_output_or_suffix() {
    for method in ["print", "panic"] {
        let source = format!(
            "d:@\"debug\";f<null>:(r<{{-><never>;tag<boolean>}}> ){{d.{method}(\"a{{r}}tail{{1}}\")}}"
        );
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let (&id, op) = checker.outputs.first_key_value().unwrap();
        assert_eq!(op.stopped, Some(1));
        assert!(op.parts[1].unwrap().primary);
        assert!(checker.points[op.parts[3].unwrap().point].complete);
        assert_eq!(
            op.edges.last().unwrap().to,
            Port::Projection { point: id, step: 1 }
        );
        assert!(!op.edges.iter().any(
            |edge| matches!(edge.to, Port::Output { part, .. } if part >= 1)
                || edge.to == Port::Operation(id)
                || edge.to == Port::Normal(id)
        ));
        assert_ne!(op.owner, 0);
    }
}

#[test]
pub(crate) fn output_primary_stages_keep_calls_scalar_results_control_and_errors() {
    let source = "d:@\"debug\";get<{-><int32>;tag<boolean>}>:(){->{->7;->tag:true}};d.print(\"{get()}{get()+1}\")";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let op = checker.outputs.values().next().unwrap();
    assert_eq!(checker.invocations.len(), 2);
    assert!(op.parts[0].unwrap().primary);
    assert!(!op.parts[1].unwrap().primary);
    let call = checker
        .invocations
        .values()
        .find(|call| call.point == op.parts[0].unwrap().point)
        .unwrap();
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    assert_eq!(
        op.edges
            .iter()
            .filter(|edge| matches!(edge.to, Port::Projection { .. }))
            .count(),
        1
    );
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;d:@\"debug\";r:{->7;->tag:true};|flag|d.print(r)")
                .unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.outputs.values().next().unwrap().control);
    assert_eq!(
        crate::compile("d:@\"debug\";n:1;r:{->&n;->tag:true};d.print(r)").unwrap_err()[0].code,
        "B001"
    );
}

#[test]
pub(crate) fn output_primary_stages_validate_flags_and_publish_atomically() {
    let (mut checker, block) = check("d:@\"debug\";r:{->7;->tag:true};d.print(r)");
    let hir::Stmt::Expr(value) = block.stmts.last().unwrap() else {
        panic!()
    };
    let hir::ExprKind::Print { parts, .. } = &value.kind else {
        panic!()
    };
    let (&id, op) = checker.outputs.first_key_value().unwrap();
    let points = op.parts.clone();
    let count = checker.output_edges;
    checker
        .output_operation(id, false, parts, points.clone(), value.span)
        .unwrap();
    assert_eq!(checker.output_edges, count);
    let mut changed = points.clone();
    changed[0].as_mut().unwrap().primary = false;
    assert!(
        checker
            .output_operation(id, false, parts, changed, value.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.outputs.clear();
    checker.output_edges = 0;
    let mut invalid = parts.clone();
    invalid[0].kind = hir::ExprKind::Int(7);
    assert!(
        checker
            .output_operation(id, false, &invalid, points.clone(), value.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.coercion_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .output_operation(id, false, parts, points, value.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.outputs.is_empty());
    assert_eq!(checker.output_edges, 0);
}
