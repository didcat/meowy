use super::super::super::tests::value;
use super::*;

pub(super) const START: Port = Port::BlockEntry(0);
pub(super) const LEFT: Port = Port::Entry(1);
pub(super) const RIGHT: Port = Port::Entry(2);
pub(super) const END: Port = Port::Normal(3);

pub(super) fn edges() -> Vec<(Family, Edge)> {
    [
        (START, LEFT, Route::True),
        (START, RIGHT, Route::False),
        (LEFT, END, Route::Join),
        (RIGHT, END, Route::Join),
    ]
    .into_iter()
    .map(|(from, to, route)| (Family::Branch, Edge::new(from, to, route)))
    .collect()
}

pub(super) fn site(port: Port) -> ports::Site {
    let at = match port {
        Port::Entry(id) | Port::Normal(id) => id,
        _ => 0,
    };
    ports::Site {
        owner: 0,
        block: 0,
        span: Span::new(at, at + 1),
    }
}

pub(super) fn graph(edges: &[(Family, Edge)]) -> graph::Graph<'_> {
    let mut graph = graph::Graph {
        edges,
        nodes: [START, LEFT, RIGHT, END]
            .into_iter()
            .map(|port| {
                (
                    port,
                    graph::Node {
                        site: site(port),
                        incoming: Vec::new(),
                        outgoing: Vec::new(),
                    },
                )
            })
            .collect(),
        external: HashMap::new(),
    };
    for (position, (_, edge)) in edges.iter().enumerate() {
        let reached = graph.nodes.contains_key(&edge.from);
        if let Some(node) = graph.nodes.get_mut(&edge.from) {
            node.outgoing.push(position);
        } else {
            graph.external.insert(edge.from, site(edge.from));
        }
        graph
            .nodes
            .get_mut(&edge.to)
            .unwrap()
            .incoming
            .push(graph::Incoming {
                edge: position,
                source: site(edge.from),
                reached,
            });
    }
    for node in graph.nodes.values_mut() {
        node.incoming.sort_by_key(|input| {
            (
                input.source.span.start,
                input.source.span.end,
                edges[input.edge].1.from,
                input.edge,
            )
        });
    }
    graph
}

pub(super) fn setup(graph: &graph::Graph<'_>) -> (Checker, Snapshots) {
    let mut checker = Checker::new();
    let mut state = State::new(0, crate::flow::TRUE, &mut checker.flow, Span::default()).unwrap();
    state
        .put(
            0,
            &value(0, 0, 7, &mut checker.flow),
            &mut checker.flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let entries = BTreeMap::from([(0, (START, state))]);
    let snapshots = checker
        .scalar_entry_snapshots(graph, &entries, Span::default(), MAX_EDGES)
        .unwrap();
    (checker, snapshots)
}

pub(super) fn publish(checker: &mut Checker, snapshots: &mut Snapshots, port: Port, value: Value) {
    snapshots
        .record(
            port,
            Row {
                owner: 0,
                before: value,
                after: value,
            },
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
}

#[test]
pub(crate) fn scalar_incoming_reuses_identical_snapshots_and_joins_distinct_histories() {
    for number in [7, 9] {
        let edges = edges();
        let graph = graph(&edges);
        let (mut checker, mut snapshots) = setup(&graph);
        let seed = Value::Known(snapshots.seeds[&START]);
        publish(&mut checker, &mut snapshots, LEFT, seed);
        publish(&mut checker, &mut snapshots, RIGHT, seed);
        let room = snapshots.room();
        assert_eq!(
            snapshots
                .incoming(&graph, END, &mut checker.flow, Span::default())
                .unwrap(),
            seed
        );
        assert_eq!(snapshots.room(), room);
        let (mut checker, mut snapshots) = setup(&graph);
        let seed = Value::Known(snapshots.seeds[&START]);
        publish(&mut checker, &mut snapshots, LEFT, seed);
        let mut next = snapshots
            .seed(START)
            .unwrap()
            .snapshot(&mut checker.flow, Span::default(), snapshots.room())
            .unwrap();
        let mut cell = value(0, 2, number, &mut checker.flow);
        cell.control = true;
        next.put(
            0,
            &cell,
            &mut checker.flow,
            Span::default(),
            snapshots.room(),
        )
        .unwrap();
        let changed = snapshots
            .copy(&next, &mut checker.flow, Span::default())
            .unwrap();
        publish(&mut checker, &mut snapshots, RIGHT, changed);
        let joined = snapshots
            .incoming(&graph, END, &mut checker.flow, Span::default())
            .unwrap();
        let Some(View::Known(state)) = snapshots.view(joined) else {
            panic!()
        };
        let cell = state.get(0).unwrap();
        let expected = Domain::integer(7, &mut checker.flow, Span::default())
            .unwrap()
            .join(
                &Domain::integer(number, &mut checker.flow, Span::default()).unwrap(),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert_eq!(
            cell.data,
            Data::Initialized {
                version: None,
                domain: Some(expected),
                origin: None
            }
        );
        assert!(cell.control);
        assert_ne!(state.get(0), snapshots.seed(START).unwrap().get(0));
        assert_eq!(snapshots.before(END), None);
    }
}

#[test]
pub(crate) fn scalar_incoming_missing_sources_do_not_become_bottom_or_hide_unfinished_inputs() {
    let mut edges = edges();
    edges.push((
        Family::ScopeExit,
        Edge::new(Port::Leave(0), END, Route::Join),
    ));
    let graph = graph(&edges);
    let (mut checker, mut snapshots) = setup(&graph);
    let seed = Value::Known(snapshots.seeds[&START]);
    publish(&mut checker, &mut snapshots, LEFT, seed);
    assert!(
        snapshots
            .incoming(&graph, END, &mut checker.flow, Span::default())
            .is_err()
    );
    publish(&mut checker, &mut snapshots, RIGHT, Value::Missing);
    assert_eq!(
        snapshots
            .incoming(&graph, END, &mut checker.flow, Span::default())
            .unwrap(),
        Value::Missing
    );
    for missing in [false, true] {
        let edges = self::edges();
        let graph = self::graph(&edges);
        let (mut checker, mut snapshots) = setup(&graph);
        let bottom = State::new(0, FALSE, &mut checker.flow, Span::default()).unwrap();
        let left = snapshots
            .copy(&bottom, &mut checker.flow, Span::default())
            .unwrap();
        let right = if missing {
            Value::Missing
        } else {
            snapshots
                .copy(&bottom, &mut checker.flow, Span::default())
                .unwrap()
        };
        publish(&mut checker, &mut snapshots, LEFT, left);
        publish(&mut checker, &mut snapshots, RIGHT, right);
        let input = snapshots
            .incoming(&graph, END, &mut checker.flow, Span::default())
            .unwrap();
        if missing {
            assert_eq!(input, Value::Missing);
        } else {
            let Some(View::Known(state)) = snapshots.view(input) else {
                panic!()
            };
            assert_eq!(state.reach, FALSE);
        }
    }
}

#[test]
pub(crate) fn scalar_incoming_rejects_stale_order_identity_and_snapshot_handles() {
    for fault in 0..4 {
        let edges = edges();
        let mut graph = graph(&edges);
        let (mut checker, mut snapshots) = setup(&graph);
        let seed = Value::Known(snapshots.seeds[&START]);
        publish(&mut checker, &mut snapshots, LEFT, seed);
        publish(&mut checker, &mut snapshots, RIGHT, seed);
        match fault {
            0 => graph.nodes.get_mut(&END).unwrap().incoming.reverse(),
            1 => graph.nodes.get_mut(&END).unwrap().incoming[0].source.owner = 1,
            2 => graph.nodes.get_mut(&END).unwrap().incoming[0].edge = usize::MAX,
            3 => snapshots.rows.get_mut(&RIGHT).unwrap().after = Value::Known(usize::MAX),
            _ => unreachable!(),
        }
        let before = format!("{:?}", snapshots.rows);
        assert_eq!(
            snapshots
                .incoming(&graph, END, &mut checker.flow, Span::default())
                .unwrap_err()
                .code,
            "B001"
        );
        assert_eq!(format!("{:?}", snapshots.rows), before);
    }
}
