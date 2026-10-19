use super::*;
use crate::check::dependencies::ScalarLiteral;

pub(super) fn context(source: &str) -> (Checker, Reports, ports::Ports) {
    let (mut checker, program, reports) = versions::tests::context(source);
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, _) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    (checker, reports, ports)
}

#[test]
pub(crate) fn scalar_observations_keep_independent_literal_stages_and_source_identity() {
    for (operation, result) in [(true, false), (false, true), (true, true)] {
        let (mut checker, mut reports, mut ports) = context("7");
        let point = *checker.scalar_leaves.first_key_value().unwrap().0;
        let (_, Effect::Scalar(observed)) = reports.effects.get_mut(&point).unwrap() else {
            panic!()
        };
        observed.operation = operation;
        observed.result = result;
        for (port, reached) in [
            (Port::Operation(point), operation),
            (Port::Normal(point), result),
        ] {
            if !reached {
                ports.remove(&port);
            }
        }
        let graph = graph::Graph {
            edges: &reports.index.edges,
            nodes: ports
                .into_iter()
                .map(|(port, site)| {
                    (
                        port,
                        graph::Node {
                            site,
                            incoming: Vec::new(),
                            outgoing: Vec::new(),
                        },
                    )
                })
                .collect(),
            external: HashMap::new(),
        };
        let definitions = definitions::Definitions::default();
        let (observed, _) = checker
            .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(
            observed.sources.contains_key(&Port::Operation(point)),
            operation
        );
        assert_eq!(observed.sources.contains_key(&Port::Normal(point)), result);
        assert!(
            observed
                .sources
                .values()
                .all(|source| std::ptr::eq(*source, &reports.effects[&point]))
        );
    }
    let (mut checker, reports, ports) = context("d:@\"debug\";7;d.panic(\"stop\");late:9");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observed, _) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    let (&late, _) = checker
        .scalar_leaves
        .iter()
        .find(|(_, leaf)| leaf.value == Some(ScalarLiteral::Int(9)))
        .unwrap();
    assert!(!observed.sources.contains_key(&Port::Operation(late)));
    assert!(!observed.sources.contains_key(&Port::Normal(late)));
}

#[test]
pub(crate) fn scalar_observations_reject_stale_stages_sites_and_late_payloads() {
    for fault in 0..6 {
        let (mut checker, mut reports, mut ports) = context("1;2");
        let point = *checker.scalar_leaves.last_key_value().unwrap().0;
        match fault {
            0 => {
                ports.remove(&Port::Normal(point));
            }
            1 => ports.get_mut(&Port::Operation(point)).unwrap().owner += 1,
            2 => ports.get_mut(&Port::Normal(point)).unwrap().span = Span::default(),
            3..=5 => {
                let (owner, Effect::Scalar(observed)) = reports.effects.get_mut(&point).unwrap()
                else {
                    panic!()
                };
                match fault {
                    3 => observed.result = false,
                    4 => observed.value = Some(ScalarLiteral::Int(9)),
                    5 => *owner += 1,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
        let graph = graph::Graph {
            edges: &reports.index.edges,
            nodes: ports
                .into_iter()
                .map(|(port, site)| {
                    (
                        port,
                        graph::Node {
                            site,
                            incoming: Vec::new(),
                            outgoing: Vec::new(),
                        },
                    )
                })
                .collect(),
            external: HashMap::new(),
        };
        let definitions = definitions::Definitions::default();
        let before = format!("{reports:?}{graph:?}");
        assert!(
            checker
                .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(before, format!("{reports:?}{graph:?}"));
    }
}

#[test]
pub(crate) fn scalar_observations_bound_rows_and_exact_work_atomically() {
    let (mut checker, reports, ports) = context("1;false;null;1.5;\"cat\"");
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
    assert_eq!(used, expected.sources.len());
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result =
            checker.scalar_observations(&reports, &graph, &definitions, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((observed, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(observed.sources, expected.sources);
        }
    }
}
