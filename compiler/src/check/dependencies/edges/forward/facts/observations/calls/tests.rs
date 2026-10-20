use super::*;

pub(super) fn context() -> (Checker, Reports, ports::Ports, Mutations) {
    let (mut checker, program, reports) = versions::tests::context("f:(n<int32>){};f(1);f(2)");
    checker.proofs.conditions = std::mem::take(&mut checker.guards);
    checker.proofs.tags = std::mem::take(&mut checker.tags);
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    let calls = checker
        .qualify_borrow_calls(&program, &reports, &facts, Span::default())
        .unwrap();
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, _) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    (checker, reports, ports, calls)
}

#[test]
pub(crate) fn scalar_call_observations_borrow_original_inputs_and_reserve_exact_work() {
    let (mut checker, reports, ports, calls) = context();
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let start = checker.flow.work;
    let (observations, left) = checker
        .scalar_call_observations(
            &reports,
            &graph,
            &calls,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let cost = checker.flow.work - start;
    assert!(std::ptr::eq(observations.calls.unwrap(), &calls));
    assert_eq!(MAX_EDGES - left, calls.len());
    let before = format!("{calls:?}{reports:?}{graph:?}");
    for (room, extra, pass) in [(1, 0, false), (2, 1, false), (2, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_call_observations(
            &reports,
            &graph,
            &calls,
            Observations::new(&definitions),
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        if let Ok((observations, left)) = result {
            assert_eq!(left, 0);
            assert!(std::ptr::eq(observations.calls.unwrap(), &calls));
        }
        assert_eq!(format!("{calls:?}{reports:?}{graph:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_call_observations_reject_stale_call_sites_owners_and_operation_edges() {
    for fault in 0..6 {
        let (mut checker, reports, ports, mut calls) = context();
        let point = *calls.keys().next_back().unwrap();
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        match fault {
            0 => calls.get_mut(&point).unwrap().site += 1,
            1 => calls.get_mut(&point).unwrap().owner += 1,
            2 => calls.get_mut(&point).unwrap().input = Port::Normal(point),
            3 => {
                graph
                    .nodes
                    .get_mut(&Port::Operation(point))
                    .unwrap()
                    .incoming[0]
                    .edge = usize::MAX
            }
            4 => {
                graph
                    .nodes
                    .get_mut(&Port::Operation(point))
                    .unwrap()
                    .incoming[0]
                    .reached = false
            }
            5 => {
                graph.nodes.remove(&Port::Operation(point));
            }
            _ => unreachable!(),
        }
        let definitions = definitions::Definitions::default();
        let before = format!("{calls:?}{reports:?}{graph:?}");
        assert!(
            checker
                .scalar_call_observations(
                    &reports,
                    &graph,
                    &calls,
                    Observations::new(&definitions),
                    Span::default(),
                    MAX_EDGES
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{calls:?}{reports:?}{graph:?}"), before);
    }
}
