use super::*;

pub(super) fn checked(source: &str) -> (Checker, crate::hir::Block) {
    let mut checker = Checker::new();
    let ast = crate::parser::parse(source).unwrap();
    let body = checker.block(&ast, None, None).unwrap();
    (checker, body)
}

#[test]
pub(crate) fn function_endpoints_connect_leading_and_interleaved_declarations() {
    for (source, index) in [("f<int32>:(){->1};x:2", 0), ("y:1;f<int32>:(){->1};x:2", 1)] {
        crate::compile(source).unwrap();
        let (mut checker, body) = checked(source);
        let items = &checker.sequences[&SequenceSource::Block(body.id)].items;
        let declaration = items[index].unwrap();
        let last = items.last().unwrap().unwrap();
        assert_eq!(
            checker.endpoints[&SequenceSource::Stmt(declaration)],
            [Edge::new(
                Port::Entry(declaration),
                Port::Normal(declaration),
                Route::Next
            )]
        );
        let graph = checker.forward_index(Span::default()).unwrap();
        let walk = graph
            .walk(
                Port::BlockEntry(body.id),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert!(walk.ports.contains(&Port::Operation(last)));
        for function in checker.functions.iter().flatten() {
            assert!(!walk.ports.contains(&Port::BlockEntry(function.body.id)));
        }
    }
}

#[test]
pub(crate) fn function_endpoints_complete_unused_never_definitions_but_not_never_calls() {
    for (tail, present) in [("x:1", true), ("stop();x:1", false)] {
        let source = format!("stop<never>:(){{'again{{'again.restart()}}}};{tail}");
        crate::compile(&source).unwrap();
        let (mut checker, body) = checked(&source);
        let last = checker.sequences[&SequenceSource::Block(body.id)]
            .items
            .last()
            .unwrap()
            .unwrap();
        let graph = checker.forward_index(Span::default()).unwrap();
        let walk = graph
            .walk(
                Port::BlockEntry(body.id),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert_eq!(walk.ports.contains(&Port::Operation(last)), present);
        assert!(!walk.ports.contains(&Port::BlockEntry(
            checker.functions[0].as_ref().unwrap().body.id
        )));
    }
}

#[test]
pub(crate) fn function_endpoints_preserve_failed_definition_diagnostics() {
    for (source, code) in [
        ("f<int32>:(){->false}", "E207"),
        ("f:(){missing}", "E201"),
        ("f:=(){->1}", "B001"),
    ] {
        let ast = crate::parser::parse(source).unwrap();
        let mut checker = Checker::new();
        assert_eq!(checker.block(&ast, None, None).unwrap_err().code, code);
        assert!(
            !checker
                .endpoints
                .keys()
                .any(|key| matches!(key, SequenceSource::Stmt(_)))
        );
    }
}

#[test]
pub(crate) fn function_endpoints_bound_atomic_publication_and_idempotent_replays() {
    let (mut checker, body) = checked("f<int32>:(){->1};x:2");
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    let key = SequenceSource::Stmt(id);
    let count = checker.endpoint_edges;
    checker
        .function_declaration_endpoint(id, 0, Span::default())
        .unwrap();
    assert_eq!(checker.endpoint_edges, count);
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    let total: usize = checker.edge_counts().iter().map(|(_, size)| size).sum();
    checker.sequence_edges += MAX_EDGES - total;
    assert!(
        checker
            .function_declaration_endpoint(id, 0, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!(checker.endpoint_edges, count - 1);
    checker.sequence_edges -= 1;
    checker
        .function_declaration_endpoint(id, 0, Span::default())
        .unwrap();
    assert_eq!(checker.endpoint_edges, count);
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    let before = checker.flow.work;
    checker
        .function_declaration_endpoint(id, 0, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .function_declaration_endpoint(id, 0, Span::default())
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!(checker.endpoint_edges, count - 1);
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .function_declaration_endpoint(id, 0, Span::default())
        .unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
}

#[test]
pub(crate) fn function_endpoints_preserve_nested_owners_and_opaque_statement_barriers() {
    let (checker, _) = checked("outer<int32>:(){inner<int32>:(){->2};->inner()};x:1");
    let owners: Vec<_> = checker
        .endpoints
        .keys()
        .filter_map(|key| {
            let SequenceSource::Stmt(id) = key else {
                return None;
            };
            Some(checker.points[*id].owner)
        })
        .collect();
    assert_eq!(owners, [0, 1]);
    for source in [
        "'out{finish:'out.leave};x:2",
        "f<()->int32>;f<int32>:(){->1};x:2",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, body) = checked(source);
        let last = checker.sequences[&SequenceSource::Block(body.id)]
            .items
            .last()
            .unwrap()
            .unwrap();
        let graph = checker.forward_index(Span::default()).unwrap();
        let walk = graph
            .walk(
                Port::BlockEntry(body.id),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert!(!walk.ports.contains(&Port::Operation(last)), "{source}");
        assert!(!walk.missing.is_empty());
    }
}

#[test]
pub(crate) fn function_endpoints_reject_missing_foreign_incomplete_and_conflicting_metadata() {
    let (mut checker, body) = checked("f<int32>:(){->1};x:2");
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    let expression = checker
        .points
        .iter()
        .position(|point| point.kind == PointKind::Expr)
        .unwrap();
    let count = checker.endpoint_edges;
    for (point, function) in [(usize::MAX, 0), (id, usize::MAX), (expression, 0)] {
        assert!(
            checker
                .function_declaration_endpoint(point, function, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    checker.points[id].complete = false;
    assert!(
        checker
            .function_declaration_endpoint(id, 0, Span::default())
            .is_err()
    );
    checker.points[id].complete = true;
    checker.owner = 1;
    assert!(
        checker
            .function_declaration_endpoint(id, 0, Span::default())
            .is_err()
    );
    checker.owner = 0;
    let body = checker.functions[0].as_ref().unwrap().body.id;
    checker.bodies.get_mut(&body).unwrap().owner = 0;
    assert!(
        checker
            .function_declaration_endpoint(id, 0, Span::default())
            .is_err()
    );
    checker.bodies.get_mut(&body).unwrap().owner = 1;
    checker
        .endpoints
        .insert(SequenceSource::Stmt(id), Vec::new());
    assert!(
        checker
            .function_declaration_endpoint(id, 0, Span::default())
            .is_err()
    );
    assert!(checker.endpoints[&SequenceSource::Stmt(id)].is_empty());
    assert_eq!(checker.endpoint_edges, count);
}
