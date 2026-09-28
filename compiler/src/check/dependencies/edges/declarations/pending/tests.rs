use super::*;

pub(super) fn checked() -> (Checker, PointId, Prepared) {
    let ast = crate::parser::parse("p:@\"proof\";r:p.can_copy<uint8>()").unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    checker.checked_stmt(&ast.stmts[0]).unwrap();
    let (point, _) = checker.checked_stmt(&ast.stmts[1]).unwrap();
    checker
        .endpoints
        .remove(&SequenceSource::Stmt(point))
        .unwrap();
    checker.endpoint_edges -= 1;
    (
        checker,
        point,
        Prepared {
            id: 0,
            created: true,
        },
    )
}

#[test]
pub(crate) fn pending_endpoints_validate_query_point_site_root_and_owner_before_publication() {
    for fault in 0..20 {
        let (mut checker, point, mut prepared) = checked();
        let origin = checker.queries[0].point;
        let site = checker.queries[0].site.unwrap();
        match fault {
            0 => prepared.id = usize::MAX,
            1 => prepared.created = false,
            2 => checker.points[point].kind = PointKind::Expr,
            3 => checker.points[point].owner = 1,
            4 => checker.points[point].complete = false,
            5 => checker.queries[0].owner = 1,
            6 => checker.queries[0].root = usize::MAX,
            7 => checker.points[origin].complete = false,
            8 => checker.points[origin].parent = None,
            9 => checker.points[origin].span = Span::default(),
            10 => checker.queries[0].site = None,
            11 => checker.sites.get_mut(&site).unwrap().owner = 1,
            12 => checker.sites.get_mut(&site).unwrap().point = Some(usize::MAX),
            13 => checker.query_budgets[0] = None,
            14 => {
                checker.query_budgets[0]
                    .as_mut()
                    .unwrap()
                    .charge(crate::check::required::MAX_STEPS + 1, 0)
                    .unwrap_err();
            }
            15 => checker.points[origin].kind = PointKind::Read,
            16 => checker.points[origin].owner = 1,
            17 => checker.points[origin].block = Some(usize::MAX),
            18 => checker.queries[0].point = usize::MAX,
            19 => checker.sites.get_mut(&site).unwrap().complete = false,
            _ => unreachable!(),
        }
        let before = checker.endpoint_edges;
        let error = checker
            .pending_declaration_endpoint(point, prepared, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001", "fault {fault}");
        assert_eq!(checker.endpoint_edges, before);
        assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(point)));
    }
}

#[test]
pub(crate) fn pending_endpoints_bound_atomic_and_idempotent_publication() {
    let (mut checker, point, prepared) = checked();
    let key = SequenceSource::Stmt(point);
    let before = checker.flow.work;
    checker
        .pending_declaration_endpoint(point, prepared, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    assert_eq!(
        checker.endpoints[&key],
        [Edge::new(
            Port::Entry(point),
            Port::Normal(point),
            Route::Next
        )]
    );
    let count = checker.endpoint_edges;
    checker
        .pending_declaration_endpoint(point, prepared, Span::default())
        .unwrap();
    assert_eq!(checker.endpoint_edges, count);
    for spare in [0, 1] {
        let (mut checker, point, prepared) = checked();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.pending_declaration_endpoint(point, prepared, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        assert_eq!(checker.endpoints.contains_key(&key), spare == 0);
    }
    let (mut checker, point, prepared) = checked();
    let count = checker.endpoint_edges;
    let total: usize = checker.edge_counts().iter().map(|(_, count)| count).sum();
    checker.sequence_edges += MAX_EDGES - total;
    assert!(
        checker
            .pending_declaration_endpoint(point, prepared, Span::default())
            .is_err()
    );
    assert_eq!(checker.endpoint_edges, count);
    assert!(!checker.endpoints.contains_key(&key));
    checker.sequence_edges -= 1;
    checker
        .pending_declaration_endpoint(point, prepared, Span::default())
        .unwrap();
    assert_eq!(checker.endpoint_edges, count + 1);
}

#[test]
pub(crate) fn pending_endpoints_connect_statements_without_running_or_recreating_queries() {
    for (prefix, reached) in [
        ("", true),
        ("stop<never>:(){'loop{'loop.restart()}};stop();", false),
    ] {
        let source = format!(
            "p:@\"proof\";{prefix}r:p.can_copy<uint8>();copy:r;(copy);p.can_copy<uint16>();x:1"
        );
        let ast = crate::parser::parse(&source).unwrap();
        let mut checker = Checker::new();
        let body = checker.block(&ast, None, None).unwrap();
        let items = checker.sequences[&SequenceSource::Block(body.id)]
            .items
            .clone();
        let graph = checker.forward_index(Span::default()).unwrap();
        let walk = graph
            .walk(
                Port::BlockEntry(body.id),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert_eq!(
            walk.ports
                .contains(&Port::Operation(items.last().unwrap().unwrap())),
            reached
        );
        assert_eq!(checker.queries.len(), 2);
        for query in &checker.queries {
            assert!(!walk.ports.contains(&Port::Entry(query.point)));
            assert!(!walk.ports.contains(&Port::Normal(query.point)));
            let stmt = checker.points[query.point].parent.unwrap();
            assert!(checker.endpoints.contains_key(&SequenceSource::Stmt(stmt)));
        }
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, "B001");
    }
}

#[test]
pub(crate) fn pending_endpoints_accept_open_roots_and_preserve_shared_query_origins() {
    let ast = crate::parser::parse("p:@\"proof\";r:p.can_copy<uint8>();{copy:r;(copy)}").unwrap();
    let mut checker = Checker::new();
    checker
        .construction_root(ast.span, |checker| {
            checker.block(&ast, None, None)?;
            assert!(checker.query_budgets[0].is_none());
            assert_eq!(checker.queries.len(), 1);
            Ok(())
        })
        .unwrap();
    assert!(checker.query_budgets[0].is_some());
    assert!(checker.type_work.is_none());
    let mut checker = Checker::new();
    let ast =
        crate::parser::parse("p:@\"proof\";f:(){r:p.can_copy<uint8>();{copy:r};x:1}").unwrap();
    checker.block(&ast, None, None).unwrap();
    let query = &checker.queries[0];
    assert_eq!(query.owner, 1);
    assert!(checker.endpoints.contains_key(&SequenceSource::Stmt(
        checker.points[query.point].parent.unwrap()
    )));
}

#[test]
pub(crate) fn pending_endpoints_keep_source_errors_before_completion() {
    for (tail, code) in [
        ("copy<p.Always>:r", "E207"),
        ("copy<int32>:r", "E223"),
        ("copy:=r", "E223"),
        ("r:r", "E203"),
        ("f:(){copy:r}", "E223"),
        ("copy:p.can_copy<Missing>()", "E202"),
        ("copy<p.Result>:p.can_copy<uint8>();bad:missing", "E201"),
    ] {
        let source = format!("p:@\"proof\";r:p.can_copy<uint8>();{tail}");
        let ast = crate::parser::parse(&source).unwrap();
        let mut checker = Checker::new();
        assert_eq!(
            checker.block(&ast, None, None).unwrap_err().code,
            code,
            "{source}"
        );
        for (id, point) in checker.points.iter().enumerate() {
            if point.kind == PointKind::Stmt && !point.complete {
                assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
            }
        }
        assert!(checker.type_work.is_none());
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, code);
    }
}
