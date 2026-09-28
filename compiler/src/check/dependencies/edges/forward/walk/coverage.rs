use super::{tests::index, *};

#[test]
pub(crate) fn structural_walk_obeys_retained_item_allowance_before_each_growth() {
    let start = Port::Entry(0);
    let graph = index(&[
        Edge::new(start, Port::Normal(0), Route::Next),
        Edge::new(start, start, Route::Backedge),
    ]);
    let expected = graph
        .walk(start, &mut crate::flow::Flow::new(), Span::default())
        .unwrap();
    assert_eq!(expected.len(), 5);
    for items in 0..expected.len() {
        let error = graph
            .walk_limited(start, &mut crate::flow::Flow::new(), Span::default(), items)
            .unwrap_err();
        assert!(error.message.contains("structural-walk budget"));
    }
    assert_eq!(
        graph
            .walk_limited(
                start,
                &mut crate::flow::Flow::new(),
                Span::default(),
                expected.len()
            )
            .unwrap(),
        expected
    );
    let empty = index(&[]);
    assert!(
        empty
            .walk_limited(start, &mut crate::flow::Flow::new(), Span::default(), 1)
            .is_err()
    );
    assert_eq!(
        empty
            .walk_limited(start, &mut crate::flow::Flow::new(), Span::default(), 2)
            .unwrap()
            .len(),
        2
    );
}

#[test]
pub(crate) fn structural_walk_retains_both_conditional_paths_and_exact_join_ports() {
    let start = Port::Entry(0);
    let left = Port::Output { point: 1, part: 0 };
    let right = Port::Output { point: 1, part: 1 };
    let join = Port::Normal(1);
    let end = Port::Normal(2);
    let edges = [
        Edge::new(start, left, Route::True),
        Edge::new(start, right, Route::False),
        Edge::new(left, join, Route::Join),
        Edge::new(right, join, Route::Checked),
        Edge::new(join, end, Route::Returned),
    ];
    let graph = index(&edges);
    let walk = graph
        .walk(start, &mut crate::flow::Flow::new(), Span::default())
        .unwrap();
    assert_eq!(walk.ports, [start, left, right, join, end]);
    assert_eq!(walk.forward, [0, 1, 2, 3, 4]);
    assert_eq!(walk.missing, [end]);
    for (position, edge) in edges.iter().enumerate() {
        assert_eq!(graph.edges[walk.forward[position]].1, *edge);
    }
}

#[test]
pub(crate) fn structural_walk_keeps_nested_restart_boundaries_and_function_owners() {
    let source = "flag:false;'outer{'inner{|flag|'outer.restart();'inner.leave()}};f<int32>:(x<int32>){->x+1};x:f(2)";
    crate::compile(source).unwrap();
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let counts = checker.edge_counts();
    let graph = checker.forward_index(ast.span).unwrap();
    let start = Port::BlockEntry(body.id);
    let walk = graph.walk(start, &mut checker.flow, ast.span).unwrap();
    let owner = checker.port_owner(start, ast.span).unwrap();
    for &port in &walk.ports {
        assert_eq!(checker.port_owner(port, ast.span).unwrap(), owner);
    }
    assert_eq!(walk.backedges.len(), 1);
    let (_, back) = graph.edges[walk.backedges[0]];
    let (&site, input) = checker.restart_inputs.first_key_value().unwrap();
    let restart = Port::Restart {
        site,
        target: input.target,
    };
    assert_eq!(
        back,
        Edge::new(restart, Port::BlockEntry(input.target), Route::Backedge)
    );
    let boundary = graph.walk(restart, &mut checker.flow, ast.span).unwrap();
    assert_eq!(boundary.ports, [restart]);
    assert!(boundary.forward.is_empty());
    assert!(boundary.missing.is_empty());
    assert_eq!(boundary.backedges, walk.backedges);
    let functions: Vec<_> = checker
        .functions
        .iter()
        .flatten()
        .map(|f| f.body.id)
        .collect();
    assert!(!functions.is_empty());
    for id in functions {
        let entry = Port::BlockEntry(id);
        assert!(!walk.ports.contains(&entry));
        let function = graph.walk(entry, &mut checker.flow, ast.span).unwrap();
        assert!(!function.forward.is_empty());
        assert!(!function.ports.contains(&start));
    }
    assert_eq!(checker.edge_counts(), counts);
}

#[test]
pub(crate) fn structural_walk_exhaustion_returns_no_partial_report_or_index_changes() {
    let start = Port::Entry(0);
    let graph = index(&[
        Edge::new(start, Port::Normal(0), Route::Next),
        Edge::new(Port::Normal(0), Port::Normal(1), Route::Checked),
    ]);
    let mut flow = crate::flow::Flow::new();
    let expected = graph.walk(start, &mut flow, Span::default()).unwrap();
    let work = flow.work;
    for room in [0, 1, work / 2, work - 1] {
        let mut flow = crate::flow::Flow::new();
        flow.work = crate::flow::MAX_PROOF_WORK - room;
        let error = graph.walk(start, &mut flow, Span::default()).unwrap_err();
        assert!(error.message.contains("structural-walk budget"));
        assert!(flow.exceeded());
        assert_eq!(
            graph
                .walk(start, &mut crate::flow::Flow::new(), Span::default())
                .unwrap(),
            expected
        );
    }
    let mut flow = crate::flow::Flow::new();
    flow.work = crate::flow::MAX_PROOF_WORK - work;
    assert_eq!(
        graph.walk(start, &mut flow, Span::default()).unwrap(),
        expected
    );
    assert_eq!(flow.work, crate::flow::MAX_PROOF_WORK);
    assert!(!flow.exceeded());
}

#[test]
pub(crate) fn structural_walk_bounds_ports_and_charges_empty_walks() {
    let edges: Vec<_> = (0..64)
        .map(|id| Edge::new(Port::Entry(id), Port::Entry(id + 1), Route::Next))
        .collect();
    let graph = index(&edges);
    let walk = graph
        .walk(
            Port::Entry(0),
            &mut crate::flow::Flow::new(),
            Span::default(),
        )
        .unwrap();
    assert_eq!(walk.ports.len(), edges.len() + 1);
    assert_eq!(walk.forward.len(), edges.len());
    assert_eq!(walk.missing, [Port::Entry(64)]);
    let graph = index(&[]);
    let mut flow = crate::flow::Flow::new();
    let walk = graph
        .walk(Port::Entry(0), &mut flow, Span::default())
        .unwrap();
    assert_eq!(walk.ports, [Port::Entry(0)]);
    assert_eq!(walk.missing, walk.ports);
    assert!(flow.work > 0);
    flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        graph
            .walk(Port::Entry(0), &mut flow, Span::default())
            .is_err()
    );
}

#[test]
pub(crate) fn structural_walk_retains_the_full_edge_capacity_and_rejects_overflow() {
    let start = Port::Entry(0);
    let edges: Vec<_> = (0..MAX_EDGES)
        .map(|id| {
            Edge::new(
                start,
                start,
                if id % 2 == 0 {
                    Route::True
                } else {
                    Route::Backedge
                },
            )
        })
        .collect();
    let graph = index(&edges);
    let walk = graph
        .walk(start, &mut crate::flow::Flow::new(), Span::default())
        .unwrap();
    assert_eq!(walk.ports, [start]);
    assert_eq!(walk.forward, (0..MAX_EDGES).step_by(2).collect::<Vec<_>>());
    assert_eq!(
        walk.backedges,
        (1..MAX_EDGES).step_by(2).collect::<Vec<_>>()
    );
    let oversized = ForwardIndex {
        edges: vec![(Family::Region, edges[0]); MAX_EDGES + 1],
        outgoing: BTreeMap::new(),
        operations: BTreeMap::new(),
    };
    assert!(
        oversized
            .walk(start, &mut crate::flow::Flow::new(), Span::default())
            .is_err()
    );
}
