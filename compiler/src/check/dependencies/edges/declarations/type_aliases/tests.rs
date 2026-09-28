use super::{super::tests::checked, *};

pub(super) fn reaches_tail(checker: &mut Checker, body: &crate::hir::Block) -> bool {
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
    walk.ports.contains(&Port::Operation(last))
}

#[test]
pub(crate) fn type_alias_endpoints_cross_local_exported_and_computed_declarations() {
    for source in [
        "<T>:<int32>;x<T>:1",
        "-><T>:<int32>;x<T>:1",
        "<T>:{<Inner>:<uint8>;-><Inner>};x<T>:1",
        "n:2;<T>:<int32[n+1]>;x<T>:[1]",
        "<F>:<(int32)->int32>;x:1",
        "c:@\"core\";<Kind>:<c.Type>;x:1",
        "p:@\"proof\";<Result>:<p.Result>;x:1",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, body) = checked(source);
        assert!(reaches_tail(&mut checker, &body), "{source}");
    }
    let (checker, _) = checked("<T>:{<Inner>:<uint8>;-><Inner>};x<T>:1");
    assert_eq!(
        checker
            .points
            .iter()
            .filter(|point| point.kind == PointKind::Stmt)
            .count(),
        2
    );
    assert_eq!(checker.locals.len(), 1);
    let (mut checker, _) = checked("f<int32>:(){<T>:<int32>;->1;x<T>:2}");
    let body = checker.functions[0].as_ref().unwrap().body.clone();
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    assert_eq!(checker.points[id].owner, 1);
    assert_eq!(
        checker.endpoints[&SequenceSource::Stmt(id)],
        [Edge::new(Port::Entry(id), Port::Normal(id), Route::Next)]
    );
    assert!(reaches_tail(&mut checker, &body));
}

#[test]
pub(crate) fn type_alias_endpoints_keep_other_erased_forms_and_stopped_paths() {
    for source in [
        "p:@\"proof\";revision:p.revision;x:1",
        "f<()->int32>;f<int32>:(){->1};<T>:<int32>;x:1",
        "stop<never>:(){'again{'again.restart()}};stop();<T>:<int32>;x:1",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, body) = checked(source);
        assert!(!reaches_tail(&mut checker, &body), "{source}");
    }
}

#[test]
pub(crate) fn type_alias_endpoints_preserve_construction_and_declaration_errors() {
    for (source, code) in [
        ("<T>:<Missing>", "E202"),
        ("<T>:<int32>;<T>:<uint8>", "E203"),
        ("<T>:<int32>;<T>:<int32[1/0]>", "E107"),
        ("d:@\"debug\";<T>:{d.print(1);-><int32>}", "E219"),
        ("n:=2;<T>:<int32[n]>", "E104"),
        ("n:=2;<T>:{size:n;-><int32[size]>}", "E211"),
        ("|false|{<T>:<int32[1/0]>}", "E107"),
        ("{-><T>:<int32>}", "B001"),
        ("<T>:<int32[({->2})]>", "B001"),
    ] {
        let mut checker = Checker::new();
        let ast = crate::parser::parse(source).unwrap();
        assert_eq!(
            checker.block(&ast, None, None).unwrap_err().code,
            code,
            "{source}"
        );
        for (id, point) in checker.points.iter().enumerate() {
            if point.kind == PointKind::Stmt && !point.complete {
                assert!(
                    !checker.endpoints.contains_key(&SequenceSource::Stmt(id)),
                    "{source}"
                );
            }
        }
        assert!(checker.type_work.is_none());
    }
    let mut checker = Checker::new();
    let input = crate::parser::parse("n:3").unwrap();
    checker.stmt(&input.stmts[0]).unwrap();
    for input in checker.inputs.values_mut() {
        input.derived = true;
    }
    let ast = crate::parser::parse("<T>:{copy:n;-><uint8[copy]>}").unwrap();
    assert_eq!(checker.block(&ast, None, None).unwrap_err().code, "E225");
    assert!(checker.endpoints.is_empty());
}

#[test]
pub(crate) fn type_alias_endpoints_preserve_required_root_costs_and_failures() {
    let ast = crate::parser::parse("<T>:<int32[1+2]>").unwrap();
    let root = Span::new(100, 150);
    for remaining in [4, 5, 6] {
        let mut checker = Checker::new();
        let result = checker.mode_root(root, true, |checker| {
            checker.type_work.as_mut().unwrap().logical.steps =
                crate::check::required::MAX_STEPS - remaining;
            let result = checker.stmt(&ast.stmts[0]);
            let work = checker.type_work.as_ref().unwrap();
            assert_eq!(work.logical.root, root);
            assert!(work.ordinary);
            if result.is_ok() {
                assert_eq!(work.logical.types, 2);
                assert_eq!(
                    work.logical.steps,
                    crate::check::required::MAX_STEPS - remaining + 5
                );
            }
            result
        });
        assert_eq!(result.is_ok(), remaining >= 5);
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, root);
            assert!(checker.endpoints.is_empty());
        } else {
            assert_eq!(checker.endpoint_edges, 1);
        }
        assert!(checker.type_work.is_none());
        assert!(checker.locals.is_empty());
    }
    let crate::ast::StmtKind::TypeAlias { ty, .. } = &ast.stmts[0].kind else {
        panic!("type alias")
    };
    let mut checker = Checker::new();
    checker.declare_type("T", ty, false, ty.span).unwrap();
    assert!(
        checker
            .points
            .iter()
            .all(|point| point.kind != PointKind::Stmt)
    );
    assert!(checker.endpoints.is_empty());
}

#[test]
pub(crate) fn type_alias_endpoints_reject_invalid_statement_metadata() {
    let (mut checker, body) = checked("<T>:<int32>;x:1");
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    let count = checker.endpoint_edges;
    let span = Span::default();
    assert!(checker.type_alias_endpoint(usize::MAX, span).is_err());
    checker.points[id].kind = PointKind::Expr;
    assert!(checker.type_alias_endpoint(id, span).is_err());
    checker.points[id].kind = PointKind::Stmt;
    checker.points[id].complete = false;
    assert!(checker.type_alias_endpoint(id, span).is_err());
    checker.points[id].complete = true;
    checker.owner = 1;
    assert!(checker.type_alias_endpoint(id, span).is_err());
    checker.owner = 0;
    checker
        .endpoints
        .insert(SequenceSource::Stmt(id), Vec::new());
    assert!(checker.type_alias_endpoint(id, span).is_err());
    assert!(checker.endpoints[&SequenceSource::Stmt(id)].is_empty());
    assert_eq!(checker.endpoint_edges, count);
}

#[test]
pub(crate) fn type_alias_endpoints_bound_atomic_edge_and_work_publication() {
    let (mut checker, body) = checked("<T>:<int32>");
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    let key = SequenceSource::Stmt(id);
    let span = Span::default();
    let count = checker.endpoint_edges;
    checker.type_alias_endpoint(id, span).unwrap();
    assert_eq!(checker.endpoint_edges, count);
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    let total: usize = checker.edge_counts().iter().map(|(_, size)| size).sum();
    checker.sequence_edges += MAX_EDGES - total;
    assert!(checker.type_alias_endpoint(id, span).is_err());
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!(checker.endpoint_edges, count - 1);
    checker.sequence_edges -= 1;
    let before = checker.flow.work;
    checker.type_alias_endpoint(id, span).unwrap();
    let work = checker.flow.work - before;
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(checker.type_alias_endpoint(id, span).is_err());
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!(checker.endpoint_edges, count - 1);
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker.type_alias_endpoint(id, span).unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
    assert_eq!(checker.endpoint_edges, count);
}
