use super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_group_observations_retain_reached_results_and_original_inputs() {
    let (mut checker, reports, ports) =
        context("((7));f:(){(false)};d:@\"debug\";d.panic(\"stop\");(9)");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observed, _) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(observed.forwarded.len(), 3);
    for (&point, group) in &checker.group_inputs {
        let port = Port::Normal(point);
        assert_eq!(
            observed.forwarded.get(&port),
            graph.nodes.contains_key(&port).then_some(&Forward {
                owner: group.owner,
                input: Port::Normal(group.input),
                control: false,
            })
        );
        assert!(!observed.forwarded.contains_key(&Port::Operation(point)));
    }
}

#[test]
pub(crate) fn scalar_group_observations_reject_stale_producers_sites_and_predecessors() {
    for fault in 0..8 {
        let (mut checker, mut reports, ports) = context("(7)");
        let point = *checker.group_inputs.first_key_value().unwrap().0;
        if fault == 0 {
            reports.effects.insert(point, (0, Effect::Unknown));
        }
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        match fault {
            1 => checker.group_inputs.get_mut(&point).unwrap().input = point,
            2 => checker.group_inputs.get_mut(&point).unwrap().owner += 1,
            3 => checker.points[point].complete = false,
            4 => checker.points[point].span = Span::default(),
            5 => graph.nodes.get_mut(&Port::Normal(point)).unwrap().incoming[0].edge = usize::MAX,
            6 => {
                graph.nodes.get_mut(&Port::Normal(point)).unwrap().incoming[0]
                    .source
                    .owner += 1
            }
            7 => graph.nodes.get_mut(&Port::Normal(point)).unwrap().incoming[0].reached = false,
            _ => {}
        }
        let definitions = definitions::Definitions::default();
        let before = format!("{reports:?}{graph:?}");
        assert!(
            checker
                .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
                .is_err(),
            "fault {fault}"
        );
        assert_eq!(before, format!("{reports:?}{graph:?}"));
    }
}

#[test]
pub(crate) fn scalar_group_observations_bound_shared_payload_and_work_without_inventing_results() {
    let (mut checker, reports, ports) = context("(1);((true))");
    let (mut graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    assert_eq!(
        used,
        checker.group_inputs.len() + expected.sources.len() + expected.forwarded.len()
    );
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result =
            checker.scalar_observations(&reports, &graph, &definitions, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((observed, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(observed.forwarded, expected.forwarded);
        }
    }
    checker.flow.work = 0;
    checker.flow.full = false;
    let point = *checker.group_inputs.first_key_value().unwrap().0;
    graph.nodes.remove(&Port::Normal(point));
    let (observed, _) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(!observed.forwarded.contains_key(&Port::Normal(point)));
}
