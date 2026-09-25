use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn dispatch_stages_connect_receiver_initialization_body_and_result_once() {
    for source in [
        "v:3.{->$}",
        "n:1;v:(&n).{->*$}",
        "r:{->n:2};v:r.{->$.n}",
        "f<int32>:(){->3};v:f().{->$}",
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        let sequence = &checker.sequences[&SequenceSource::Block(op.block)];
        assert_eq!(sequence.items[0], None);
        assert_eq!(
            op.edges,
            [
                Edge::new(Port::Entry(id), Port::BlockEntry(op.block), Route::Next),
                Edge::new(
                    Port::BlockEntry(op.block),
                    Port::Entry(op.input),
                    Route::Next
                ),
                Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
                Edge::new(
                    Port::Operation(id),
                    Port::Entry(sequence.items[1].unwrap()),
                    Route::Next
                ),
                Edge::new(Port::BlockResult(op.block), Port::Normal(id), Route::Result),
            ]
        );
        assert!(checker.proofs.receivers.contains(&op.local));
        assert!(!checker.endpoints.contains_key(&SequenceSource::Expr(id)));
        assert!(
            !checker.endpoints[&SequenceSource::Block(op.block)]
                .iter()
                .any(|edge| edge.from == Port::BlockEntry(op.block))
        );
        if let Some(call) = checker.invocations.values().next() {
            assert_eq!(call.point, op.input);
            assert_eq!(call.edges.last().unwrap().route, Route::Returned);
        }
    }
}

#[test]
pub(crate) fn dispatch_stages_preserve_empty_forward_and_stopped_prefixes() {
    let (checker, _) = check("v:3.{}");
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    assert!(op.edges.contains(&Edge::new(
        Port::Operation(id),
        Port::BlockNormal(op.block),
        Route::Next
    )));
    let source = "v:3.{f<()->int32>;f<int32>:(){->1};->$}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    assert_eq!(
        checker.sequences[&SequenceSource::Block(op.block)].items[1],
        None
    );
    assert!(!op.edges.iter().any(|edge| edge.from == Port::Operation(id)));
    for (tail, stopped) in [("stop().{}", true), ("3.{stop()}", false)] {
        let source = format!("d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};v:{tail}");
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        assert!(!op.edges.iter().any(|edge| edge.to == Port::Normal(id)));
        assert_eq!(op.edges.len(), if stopped { 2 } else { 4 });
    }
}

#[test]
pub(crate) fn dispatch_stages_keep_nested_owners_control_and_composed_metadata() {
    let source = "flag:false;|flag|v:2.{inner:3.{->$};->$};f<int32>:(n<int32>){->n.{->$}}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert_eq!(checker.dispatch_ops.len(), 3);
    assert!(checker.dispatch_ops.values().any(|op| op.owner != 0));
    assert!(
        checker
            .dispatch_ops
            .values()
            .any(|op| op.owner == 0 && op.control)
    );
    let source = "f<{n<int32>}>:(){->3.{->n:$}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    assert!(!checker.proofs.dispatches.is_empty());
    assert_eq!(checker.dispatch_ops.len(), 1);
    for (source, code) in [
        ("v:3.{->&$}", "E303"),
        ("n:=1;v:(&n).{n=2;->*$}", "E302"),
        ("n:=1;v:(&!n).{}", "B001"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
}

#[test]
pub(crate) fn dispatch_stages_validate_captured_ids_and_publish_with_shared_budgets() {
    let (mut checker, block) = check("v:3.{->$}");
    let hir::Stmt::Bind { value, .. } = &block.stmts[0] else {
        panic!()
    };
    let hir::ExprKind::Block(body) = &value.kind else {
        panic!()
    };
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    let op = op.clone();
    let count = checker.dispatch_edges;
    checker
        .dispatch_operation(id, op.input, op.local, body, op.span)
        .unwrap();
    assert_eq!(checker.dispatch_edges, count);
    checker.dispatch_ops.clear();
    checker.dispatch_edges = 0;
    for case in 0..3 {
        let mut body = body.clone();
        let local = if case == 0 { usize::MAX } else { op.local };
        if case == 1 {
            checker.points[op.input].parent = None;
        }
        if case == 2 {
            let hir::Stmt::Bind { value, .. } = &mut body.stmts[0] else {
                panic!()
            };
            value.ty = hir::Type::Bool;
        }
        assert!(
            checker
                .dispatch_operation(id, op.input, local, &body, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.points[op.input].parent = Some(id);
    }
    let key = SequenceSource::Block(op.block);
    let next = checker.sequences[&key].items[1].unwrap();
    checker.sequences.get_mut(&key).unwrap().items[1] = Some(usize::MAX);
    assert!(
        checker
            .dispatch_operation(id, op.input, op.local, body, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.sequences.get_mut(&key).unwrap().items[1] = Some(next);
    checker.typed_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .dispatch_operation(id, op.input, op.local, body, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.dispatch_ops.is_empty());
    assert_eq!(checker.dispatch_edges, 0);
}
