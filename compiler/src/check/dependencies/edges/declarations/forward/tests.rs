use super::*;

pub(super) fn checked() -> (Checker, PointId, Vec<crate::hir::FunctionId>) {
    let ast = crate::parser::parse("f<()->never>;g<()->int32>;g<int32>:(){inner<int32>:(){->1};->inner()};f<never>:(){'loop{'loop.restart()}}").unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    let (point, group) = checker.forward_point(&ast.stmts, 0).unwrap();
    assert_eq!(checker.points[point].span, ast.stmts[0].span);
    assert!(checker.points[point].complete);
    assert_eq!(group.end, ast.stmts.len());
    (checker, point, group.functions)
}

#[test]
pub(crate) fn forward_group_endpoints_complete_only_the_checked_group() {
    let (mut checker, point, functions) = checked();
    let key = SequenceSource::Stmt(point);
    checker
        .forward_group_endpoint(point, &functions, Span::default())
        .unwrap();
    assert_eq!(
        checker.endpoints[&key],
        [Edge::new(
            Port::Entry(point),
            Port::Normal(point),
            Route::Next
        )]
    );
    assert_eq!(functions.len(), 2);
    assert_eq!(checker.functions.len(), 3);
    assert!(
        functions
            .iter()
            .all(|id| checker.functions[*id].as_ref().unwrap().body.id
                != checker.points[point].block.unwrap())
    );
    assert!(checker.point.is_none());
    let before = checker.endpoint_edges;
    checker
        .forward_group_endpoint(point, &functions, Span::default())
        .unwrap();
    assert_eq!(checker.endpoint_edges, before);
}

#[test]
pub(crate) fn forward_group_endpoints_preserve_failed_group_checks() {
    for (source, code) in [
        ("f<()->int32>", "E221"),
        (
            "f<()->int32>;g<()->int32>;f<int32>:(){->1};g<int32>:(){->false}",
            "E207",
        ),
        ("f<()->int32>;f<boolean>:(){->true}", "E221"),
    ] {
        let mut checker = Checker::new();
        let ast = crate::parser::parse(source).unwrap();
        assert_eq!(checker.block(&ast, None, None).unwrap_err().code, code);
        for (id, point) in checker.points.iter().enumerate() {
            if point.owner == 0 && point.kind == PointKind::Stmt && !point.complete {
                assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
            }
        }
    }
}

#[test]
pub(crate) fn forward_group_endpoints_validate_all_bodies_before_publication() {
    let (mut checker, point, functions) = checked();
    let key = SequenceSource::Stmt(point);
    let count = checker.endpoint_edges;
    let span = Span::default();
    for ids in [
        vec![],
        vec![functions[0], functions[0]],
        vec![functions[1], functions[0]],
        vec![usize::MAX],
    ] {
        assert!(checker.forward_group_endpoint(point, &ids, span).is_err());
        assert!(!checker.endpoints.contains_key(&key));
    }
    let last = *functions.last().unwrap();
    let item = checker.functions[last].take();
    assert!(
        checker
            .forward_group_endpoint(point, &functions, span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    checker.functions[last] = item;
    let body = checker.functions[last].as_ref().unwrap().body.id;
    let owner = checker.bodies[&body].owner;
    checker.bodies.get_mut(&body).unwrap().owner = 0;
    assert!(
        checker
            .forward_group_endpoint(point, &functions, span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    checker.bodies.get_mut(&body).unwrap().owner = owner;
    checker.points[point].kind = PointKind::Expr;
    assert!(
        checker
            .forward_group_endpoint(point, &functions, span)
            .is_err()
    );
    checker.points[point].kind = PointKind::Stmt;
    checker.owner = 99;
    assert!(
        checker
            .forward_group_endpoint(point, &functions, span)
            .is_err()
    );
    checker.owner = 0;
    assert_eq!(checker.endpoint_edges, count);
    checker
        .forward_group_endpoint(point, &functions, span)
        .unwrap();
}

#[test]
pub(crate) fn forward_group_endpoints_bound_whole_group_work_and_edge_publication() {
    let (mut checker, point, functions) = checked();
    let key = SequenceSource::Stmt(point);
    let span = Span::default();
    let count = checker.endpoint_edges;
    assert!(
        checker
            .forward_group_endpoint(point, &vec![functions[0]; crate::flow::MAX_NODES + 1], span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    let total: usize = checker.edge_counts().iter().map(|(_, size)| size).sum();
    checker.sequence_edges += MAX_EDGES - total;
    assert!(
        checker
            .forward_group_endpoint(point, &functions, span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    checker.sequence_edges -= 1;
    let before = checker.flow.work;
    checker
        .forward_group_endpoint(point, &functions, span)
        .unwrap();
    let work = checker.flow.work - before;
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .forward_group_endpoint(point, &functions, span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!(checker.endpoint_edges, count);
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .forward_group_endpoint(point, &functions, span)
        .unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
    assert_eq!(checker.endpoint_edges, count + 1);
}
