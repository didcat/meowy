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
pub(crate) fn scalar_guard_inputs_keep_exact_conditions_and_independent_owners() {
    let (mut checker, reports, ports) =
        context("flag:=false;|flag|1;flag&&true;f:(b<boolean>){b||false}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let (inputs, parts) = checker
        .scalar_guard_inputs(&graph, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(inputs.len(), 3);
    assert_eq!(parts, MAX_EDGES - 3);
    for (&port, input) in &inputs {
        let condition = checker.points[input.source.input].parent.unwrap();
        assert_eq!(port, Port::Normal(condition));
        assert_eq!(checker.points[condition].parent, Some(input.point));
        assert_eq!(input.source, checker.branch_inputs[&input.point]);
    }
    assert!(inputs.values().any(|input| input.source.owner == 1));
}

#[test]
pub(crate) fn scalar_guard_inputs_do_not_invent_missing_or_stopped_observations() {
    for source in [
        "|true|1",
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};stop()&&true;|true|1",
    ] {
        let (mut checker, reports, ports) = context(source);
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        for edges in checker.branch_edges.values() {
            graph.nodes.remove(&edges[1].from);
        }
        assert!(
            checker
                .scalar_guard_inputs(&graph, Span::default(), MAX_EDGES)
                .unwrap()
                .0
                .is_empty()
        );
        checker.branch_inputs.clear();
        assert!(
            checker
                .scalar_guard_inputs(&graph, Span::default(), MAX_EDGES)
                .unwrap()
                .0
                .is_empty()
        );
    }
}

#[test]
pub(crate) fn scalar_guard_inputs_reject_stale_sources_cycles_and_graph_links() {
    for fault in 0..12 {
        let (mut checker, reports, ports) = context("|true|1;|false|2");
        let (&id, &branch) = checker
            .branch_inputs
            .iter()
            .min_by_key(|(id, _)| *id)
            .unwrap();
        let condition = checker.points[branch.input].parent.unwrap();
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let port = Port::Normal(condition);
        match fault {
            0 => checker.points[branch.input].parent = Some(branch.input),
            1 => checker.points[branch.input].complete = false,
            2 => checker.branch_inputs.get_mut(&id).unwrap().owner += 1,
            3 => checker.branch_inputs.get_mut(&id).unwrap().normal = false,
            4 => checker.points[id].complete = false,
            5 => {
                checker.region_edges.remove(&condition);
            }
            6 => {
                checker.branch_edges.remove(&id);
            }
            7 => checker.branch_edges.get_mut(&id).unwrap()[1].route = Route::False,
            8 => graph.nodes.get_mut(&port).unwrap().incoming[0].reached = false,
            9 => graph.nodes.get_mut(&port).unwrap().incoming[0].edge = usize::MAX,
            10 => graph.nodes.get_mut(&port).unwrap().outgoing.clear(),
            11 => graph.nodes.get_mut(&port).unwrap().site.owner += 1,
            _ => unreachable!(),
        }
        let before = format!("{graph:?}");
        assert!(
            checker
                .scalar_guard_inputs(&graph, Span::default(), MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{graph:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_guard_inputs_reserve_exact_payload_and_work_without_partial_results() {
    let (mut checker, reports, ports) = context("|true|1;|false|2");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let start = checker.flow.work;
    let (expected, _) = checker
        .scalar_guard_inputs(&graph, Span::default(), 2)
        .unwrap();
    let work = checker.flow.work - start;
    for (parts, extra, pass) in [(1, 0, false), (2, 1, false), (2, 0, true)] {
        checker.flow.full = false;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + extra;
        let result = checker.scalar_guard_inputs(&graph, Span::default(), parts);
        assert_eq!(result.is_ok(), pass);
        if pass {
            assert_eq!(result.unwrap(), (expected.clone(), 0));
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
    }
}
