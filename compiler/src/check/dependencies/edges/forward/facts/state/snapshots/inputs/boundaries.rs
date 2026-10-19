use super::super::super::super::versions::tests::context;
use super::super::super::tests::value;
use super::{tests::*, *};

pub(super) fn distinct(graph: &graph::Graph<'_>) -> (Checker, Snapshots) {
    let (mut checker, mut snapshots) = setup(graph);
    let seed = Value::Known(snapshots.seeds[&START]);
    publish(&mut checker, &mut snapshots, LEFT, seed);
    let mut next = snapshots
        .seed(START)
        .unwrap()
        .snapshot(&mut checker.flow, Span::default(), snapshots.room())
        .unwrap();
    next.put(
        0,
        &value(0, 2, 9, &mut checker.flow),
        &mut checker.flow,
        Span::default(),
        snapshots.room(),
    )
    .unwrap();
    let changed = snapshots
        .copy(&next, &mut checker.flow, Span::default())
        .unwrap();
    publish(&mut checker, &mut snapshots, RIGHT, changed);
    (checker, snapshots)
}

#[test]
pub(crate) fn scalar_incoming_bounds_exact_join_work_and_shared_payload() {
    let edges = edges();
    let graph = graph(&edges);
    let (mut checker, mut snapshots) = distinct(&graph);
    let start = checker.flow.work;
    let room = snapshots.room();
    let value = snapshots
        .incoming(&graph, END, &mut checker.flow, Span::default())
        .unwrap();
    let cost = checker.flow.work - start;
    let used = room - snapshots.room();
    let Some(View::Known(expected)) = snapshots.view(value) else {
        panic!()
    };
    assert!(used > 0);
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        let (mut checker, mut snapshots) = distinct(&graph);
        let before = format!("{:?}{:?}", snapshots.rows, snapshots.states);
        let len = snapshots.states.len();
        snapshots.parts = room;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = snapshots.incoming(&graph, END, &mut checker.flow, Span::default());
        assert_eq!(result.is_ok(), pass);
        if let Ok(value) = result {
            assert_eq!(snapshots.room(), 0);
            assert_eq!(snapshots.view(value), Some(View::Known(expected)));
        }
        assert_eq!(
            format!("{:?}{:?}", snapshots.rows, &snapshots.states[..len]),
            before
        );
        assert_eq!(snapshots.before(END), None);
    }
}

#[test]
pub(crate) fn scalar_incoming_does_not_treat_branch_routes_as_guard_authority() {
    let mut edges = edges();
    edges[2].1.route = Route::True;
    edges[3].1.route = Route::False;
    let graph = graph(&edges);
    let (mut checker, mut snapshots) = distinct(&graph);
    let value = snapshots
        .incoming(&graph, END, &mut checker.flow, Span::default())
        .unwrap();
    let Some(View::Known(state)) = snapshots.view(value) else {
        panic!()
    };
    let Data::Initialized {
        domain: Some(domain),
        ..
    } = &state.get(0).unwrap().data
    else {
        panic!()
    };
    assert_ne!(
        *domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    assert_ne!(
        *domain,
        Domain::integer(9, &mut checker.flow, Span::default()).unwrap()
    );
    assert_eq!(state.reach, crate::flow::TRUE);
}

#[test]
pub(crate) fn scalar_incoming_never_reuses_a_completed_restart_as_header_evidence() {
    let (mut checker, program, reports) =
        context("forever<never>:(n<int32>) 'loop {'loop.restart()}");
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let mut snapshots = checker
        .scalar_entry_snapshots(&graph, &entries, Span::default(), parts)
        .unwrap();
    let edge = graph
        .edges
        .iter()
        .find(|(_, edge)| edge.route == Route::Backedge)
        .unwrap()
        .1;
    let seed = Value::Known(snapshots.seeds[&edge.to]);
    snapshots
        .record(
            edge.from,
            Row {
                owner: 1,
                before: seed,
                after: seed,
            },
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    assert_eq!(
        snapshots
            .incoming(&graph, edge.to, &mut checker.flow, Span::default())
            .unwrap(),
        Value::Missing
    );
    assert_eq!(snapshots.before(edge.to), Some(View::Missing));
    assert_eq!(snapshots.seed(edge.to).unwrap().reach, crate::flow::TRUE);
}
