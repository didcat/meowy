use super::*;
use crate::check::dependencies::edges::forward::facts::observations::tests::context;

#[test]
pub(crate) fn scalar_conversion_observations_keep_exact_sources_and_sparse_stages() {
    for (operation, result) in [(true, true), (false, true), (true, false)] {
        let (mut checker, mut reports, ports) = context("n:7;v<int32><null>:n");
        let point = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.scalar.is_some())
            .unwrap()
            .0;
        let (_, Effect::Coercion(op)) = reports.effects.get_mut(&point).unwrap() else {
            panic!()
        };
        op.operation = operation;
        op.result = result;
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        for (port, reached) in [
            (Port::Operation(point), operation),
            (Port::Normal(point), result),
        ] {
            if !reached {
                let node = graph.nodes.remove(&port).unwrap();
                graph.external.insert(port, node.site);
                for node in graph.nodes.values_mut() {
                    for input in &mut node.incoming {
                        if graph.edges[input.edge].1.from == port {
                            input.reached = false;
                        }
                    }
                }
            }
        }
        let definitions = definitions::Definitions::default();
        let (obs, _) = checker
            .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(obs.sources.contains_key(&Port::Operation(point)), operation);
        assert_eq!(obs.sources.contains_key(&Port::Normal(point)), result);
        assert!(!obs.forwarded.contains_key(&Port::Normal(point)));
        if operation {
            assert!(std::ptr::eq(
                obs.sources[&Port::Operation(point)],
                &reports.effects[&point]
            ));
        }
    }
}

#[test]
pub(crate) fn scalar_conversion_observations_reject_stale_types_stages_and_edges() {
    for fault in 0..5 {
        let (mut checker, mut reports, ports) = context("n:7;v<int32><null>:n");
        let point = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.scalar.is_some())
            .unwrap()
            .0;
        let (_, Effect::Coercion(op)) = reports.effects.get_mut(&point).unwrap() else {
            panic!()
        };
        match fault {
            0 => {
                op.scalar.as_mut().unwrap().source.kind =
                    crate::check::dependencies::ScalarKind::Null
            }
            1 => op.operation = false,
            2 => op.projected = true,
            _ => (),
        }
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        if fault == 3 {
            graph
                .nodes
                .get_mut(&Port::Operation(point))
                .unwrap()
                .incoming[0]
                .edge = usize::MAX;
        }
        if fault == 4 {
            graph.nodes.get_mut(&Port::Normal(point)).unwrap().incoming[0]
                .source
                .owner += 1;
        }
        let saved = format!("{reports:?}{graph:?}");
        let definitions = definitions::Definitions::default();
        assert!(
            checker
                .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{graph:?}"), saved);
    }
}

#[test]
pub(crate) fn scalar_conversion_observations_share_exact_work_and_payload_limits() {
    let (mut checker, reports, ports) = context("a<boolean><null>:true;b<uint64><null>:null");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    let saved = format!("{reports:?}{graph:?}");
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result =
            checker.scalar_observations(&reports, &graph, &definitions, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((obs, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(obs.sources, expected.sources);
        }
        assert_eq!(format!("{reports:?}{graph:?}"), saved);
    }
}
