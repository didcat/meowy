use super::*;

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
pub(crate) fn scalar_call_results_qualify_original_signatures_without_callee_specialization() {
    let (mut checker, reports, ports) = context(
        "d:@\"debug\";f<boolean>:(){->true};alias:f;alias();recur<int32>:(n<int32>){->recur(n)};fail<int32>:(){d.panic(\"stop\")};fail();stop<never>:(){d.panic(\"stop\")};stop();f()",
    );
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observations, left) = checker
        .scalar_call_result_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert_eq!(observations.sources.len(), 3);
    assert_eq!(MAX_EDGES - left, 6);
    assert!(observations.calls.is_none());
    for call in checker.invocations.values() {
        let port = Port::Normal(call.point);
        assert_eq!(
            observations.sources.contains_key(&port),
            graph.nodes.contains_key(&port)
        );
        assert!(
            !observations
                .sources
                .contains_key(&Port::Operation(call.point))
        );
        if let Some(source) = observations.sources.get(&port) {
            assert!(std::ptr::eq(*source, &reports.effects[&call.point]));
            assert_eq!(observations.call_controls[&port], Some(false));
        }
    }
}

#[test]
pub(crate) fn scalar_call_results_keep_normal_only_observations_independent_of_operations() {
    let (mut checker, mut reports, ports) = context("f<boolean>:(){->true};f()");
    let point = checker.invocations.values().next().unwrap().point;
    let Effect::Call { operation, .. } = &mut reports.effects.get_mut(&point).unwrap().1 else {
        panic!()
    };
    *operation = false;
    let (mut graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let removed = graph.nodes.remove(&Port::Operation(point)).unwrap();
    graph.external.insert(Port::Operation(point), removed.site);
    graph.nodes.get_mut(&Port::Normal(point)).unwrap().incoming[0].reached = false;
    let definitions = definitions::Definitions::default();
    let (observations, _) = checker
        .scalar_call_result_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert!(observations.calls.is_none());
    assert_eq!(observations.sources.len(), 1);
    assert!(observations.sources.contains_key(&Port::Normal(point)));
}

#[test]
pub(crate) fn scalar_call_results_reject_stale_edges_shapes_owners_and_duplicate_observations() {
    for fault in 0..6 {
        let (mut checker, mut reports, ports) = context("f<boolean>:(){->true};f()");
        let point = checker.invocations.values().next().unwrap().point;
        if fault == 0 {
            let Effect::Call { shape, .. } = &mut reports.effects.get_mut(&point).unwrap().1 else {
                panic!()
            };
            *shape = Shape::Scalar(crate::check::dependencies::ScalarKind::Null);
        }
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let node = graph.nodes.get_mut(&Port::Normal(point)).unwrap();
        match fault {
            1 => node.incoming[0].edge = usize::MAX,
            2 => node.incoming[0].reached = false,
            3 => node.site.owner += 1,
            4 => {
                node.incoming.clear();
            }
            _ => (),
        }
        let definitions = definitions::Definitions::default();
        let mut observations = Observations::new(&definitions);
        if fault == 5 {
            observations.call_controls.insert(Port::Normal(point), None);
        }
        let before = format!("{reports:?}{graph:?}");
        assert!(
            checker
                .scalar_call_result_observations(
                    &reports,
                    &graph,
                    observations,
                    Span::default(),
                    MAX_EDGES,
                )
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{graph:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_call_results_qualify_entry_control_without_argument_authority() {
    for expected in [Some(false), Some(true), None] {
        let (mut checker, reports, ports) = context("f<boolean>:(n<boolean>){->n};|true|f(false)");
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let point = checker.invocations.values().next().unwrap().point;
        if let Some(control) = expected {
            checker.branch_inputs.values_mut().next().unwrap().control = control;
        } else {
            checker.branch_inputs.clear();
        }
        let definitions = definitions::Definitions::default();
        let (observations, _) = checker
            .scalar_call_result_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        assert_eq!(observations.call_controls[&Port::Normal(point)], expected);
    }
}

#[test]
pub(crate) fn scalar_call_results_reserve_exact_observation_work_and_payload() {
    let (mut checker, reports, ports) = context("f<boolean>:(){->true};f();f()");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let start = checker.flow.work;
    let (_, left) = checker
        .scalar_call_result_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    assert_eq!(used, 4);
    let before = format!("{reports:?}{graph:?}");
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_call_result_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        if let Ok((observations, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(observations.call_controls.len(), 2);
        }
        assert_eq!(format!("{reports:?}{graph:?}"), before);
    }
}
