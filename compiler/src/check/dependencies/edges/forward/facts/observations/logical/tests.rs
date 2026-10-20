use super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_logical_inputs_keep_operands_separate_from_matchers_and_owners() {
    let (mut checker, reports, ports) = context("|true|1;false&&true;f:(b<boolean>){b||false}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let (guards, _) = checker
        .scalar_guard_inputs(&graph, Span::default(), MAX_EDGES)
        .unwrap();
    let (logical, parts) = checker
        .scalar_logical_observations(&guards, &graph, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(logical.len(), 2);
    assert_eq!(parts, MAX_EDGES - 2);
    for (&port, result) in &logical {
        let input = guards[&Port::Normal(result.condition)];
        assert_eq!(port, Port::Normal(input.point));
        assert_eq!(result.or, checker.points[input.point].kind == PointKind::Or);
        assert_eq!(checker.points[result.right].owner, input.source.owner);
        assert_eq!(
            checker.points[result.right].parent,
            Some(input.arms[usize::from(result.or)])
        );
        assert_ne!(result.right, input.source.input);
    }
    assert!(
        logical
            .values()
            .any(|result| checker.points[result.right].owner == 1)
    );
}

#[test]
pub(crate) fn scalar_logical_inputs_preserve_missing_and_stopped_ports() {
    for source in [
        "false&&true",
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};false&&stop()",
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};stop()||true",
    ] {
        let (mut checker, reports, ports) = context(source);
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let (guards, _) = checker
            .scalar_guard_inputs(&graph, Span::default(), MAX_EDGES)
            .unwrap();
        let before = format!("{graph:?}");
        let (logical, _) = checker
            .scalar_logical_observations(&guards, &graph, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(format!("{graph:?}"), before);
        for (&port, result) in &logical {
            if source.contains("false&&stop()") {
                assert!(!graph.nodes.contains_key(&Port::Normal(result.right)));
            }
            graph.nodes.remove(&port);
        }
        assert!(
            checker
                .scalar_logical_observations(&guards, &graph, Span::default(), MAX_EDGES)
                .unwrap()
                .0
                .is_empty()
        );
        if source.contains("stop()||true") {
            assert!(logical.is_empty());
        }
    }
}

#[test]
pub(crate) fn scalar_logical_inputs_reject_stale_operands_and_join_edges() {
    for fault in 0..9 {
        let (mut checker, reports, ports) = context("true&&false");
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let (guards, _) = checker
            .scalar_guard_inputs(&graph, Span::default(), MAX_EDGES)
            .unwrap();
        let input = *guards.values().next().unwrap();
        let arm = input.arms[0];
        let Port::Entry(right) = checker.region_edges[&arm][0].to else {
            panic!()
        };
        let port = Port::Normal(input.point);
        match fault {
            0 => checker.points[right].parent = Some(input.point),
            1 => checker.points[right].owner += 1,
            2 => checker.points[right].complete = false,
            3 => {
                checker.region_edges.get_mut(&arm).unwrap()[1].from =
                    Port::Normal(input.source.input)
            }
            4 => graph
                .nodes
                .get_mut(&port)
                .unwrap()
                .incoming
                .pop()
                .map(|_| ())
                .unwrap(),
            5 => graph.nodes.get_mut(&port).unwrap().incoming[0].reached = false,
            6 => graph.nodes.get_mut(&port).unwrap().incoming[0].edge = usize::MAX,
            7 => graph.nodes.get_mut(&Port::Normal(arm)).unwrap().site.owner += 1,
            8 => {
                graph.nodes.get_mut(&Port::Entry(right)).unwrap().incoming[0]
                    .source
                    .span = Span::default()
            }
            _ => unreachable!(),
        }
        let before = format!("{graph:?}{guards:?}");
        assert!(
            checker
                .scalar_logical_observations(&guards, &graph, Span::default(), MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{graph:?}{guards:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_logical_inputs_bound_payload_and_work_atomically() {
    let (mut checker, reports, ports) = context("true&&false;false||true");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let (guards, _) = checker
        .scalar_guard_inputs(&graph, Span::default(), MAX_EDGES)
        .unwrap();
    let start = checker.flow.work;
    let (expected, _) = checker
        .scalar_logical_observations(&guards, &graph, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let before = format!("{graph:?}{guards:?}");
    for (room, extra, pass) in [(1, 0, false), (2, 1, false), (2, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_logical_observations(&guards, &graph, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((logical, parts)) = result {
            assert_eq!(parts, 0);
            assert_eq!(logical, expected);
        }
        assert_eq!(format!("{graph:?}{guards:?}"), before);
    }
}
