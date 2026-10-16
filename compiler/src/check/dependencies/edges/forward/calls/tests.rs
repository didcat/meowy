use super::*;

pub(super) fn checked(source: &str) -> (Checker, Reports) {
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let mut program = hir::Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    program.functions.reverse();
    let reports = checker.entry_reports(&program, ast.span).unwrap();
    (checker, reports)
}

#[test]
pub(crate) fn call_graph_keeps_exact_sites_alias_targets_and_independent_owners() {
    let (checker, reports) =
        checked("f<int32>:(n<int32>){->f(n)};g<int32>:(){->1};alias:g;x:alias();y:g()");
    let graph = &reports.calls;
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.sites.len(), 3);
    for (&site, call) in &graph.sites {
        let source = &checker.invocations[&site];
        assert_eq!(call.point, source.point);
        assert_eq!(call.caller, source.owner);
        assert_eq!(call.callee, source.function + 1);
        assert!(graph.nodes[&call.caller].targets[&call.callee].contains(&site));
        assert_eq!(
            graph.nodes[&call.caller].body,
            reports.entries[&call.caller].0
        );
    }
    assert_eq!(graph.nodes[&0].targets[&2].len(), 2);
    assert_eq!(graph.nodes[&1].targets[&1].len(), 1);
    assert!(graph.nodes[&2].targets.is_empty());
}

#[test]
pub(crate) fn call_graph_requires_operation_observations_and_rejects_empty_reports() {
    for (operation, result) in [(true, false), (false, true), (true, true), (false, false)] {
        let (mut checker, mut reports) = checked("f<int32>:(n<int32>){->f(n)};alias:f;x:alias(1)");
        for (_, effect) in reports.effects.values_mut() {
            if let Effect::Call {
                operation: op,
                result: end,
                ..
            } = effect
            {
                *op = operation;
                *end = result;
            }
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let graph = checker.call_graph(&reports, Span::default());
        if operation {
            assert_eq!(graph.unwrap(), reports.calls);
        } else if result {
            let graph = graph.unwrap();
            assert_eq!(graph.nodes.len(), reports.calls.nodes.len());
            assert!(graph.sites.is_empty());
            for (owner, node) in &graph.nodes {
                assert!(node.targets.is_empty());
                assert_eq!(node.body, reports.calls.nodes[owner].body);
                assert_eq!(node.missing, reports.calls.nodes[owner].missing);
                assert_eq!(node.backedges, reports.calls.nodes[owner].backedges);
            }
        } else {
            assert!(graph.unwrap_err().message.contains("call-effect identity"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn call_graph_rejects_result_only_reports_without_a_caller_entry() {
    let (mut checker, mut reports) = checked("g<int32>:(){->1};f<int32>:(){->g()};x:0");
    let call = checker.invocations.first_key_value().unwrap().1;
    assert_ne!(call.owner, 0);
    let Effect::Call { operation, .. } = &mut reports.effects.get_mut(&call.point).unwrap().1
    else {
        panic!()
    };
    *operation = false;
    reports.entries.remove(&call.owner);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let error = checker.call_graph(&reports, Span::default()).unwrap_err();
    assert!(error.message.contains("call-graph identity"));
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn call_graph_preserves_empty_entries_stopped_calls_and_exact_boundaries() {
    let (_, empty) = checked("");
    assert_eq!(empty.calls.nodes.len(), 1);
    assert!(empty.calls.sites.is_empty());
    assert!(!empty.calls.nodes[&0].missing.is_empty());
    let source = "stop<never>:(){'loop{'loop.restart()}};f<int32>:(n<int32>){->n};f(stop())";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source);
    assert_eq!(reports.calls.sites.len(), 1);
    let call = checker
        .invocations
        .values()
        .find(|call| !call.args.is_empty())
        .unwrap();
    assert!(!reports.calls.sites.contains_key(&call.site));
    for (&owner, node) in &reports.calls.nodes {
        let walk = &reports.entries[&owner].1;
        assert_eq!(node.missing, walk.missing);
        assert_eq!(node.backedges, walk.backedges);
    }
    assert!(!reports.calls.nodes[&1].backedges.is_empty());
}

#[test]
pub(crate) fn call_graph_rejects_invalid_entries_sites_and_boundary_owners() {
    for fault in 0..8 {
        let (mut checker, mut reports) = checked("f<int32>:(){->1};x:f()");
        let (&site, call) = checker.invocations.first_key_value().unwrap();
        let point = call.point;
        let callee = reports.entries[&1].0;
        match fault {
            0 => {
                reports.entries.remove(&0);
            }
            1 => {
                reports.entries.remove(&1);
            }
            2 => reports.entries.get_mut(&0).unwrap().0 = usize::MAX,
            3 => reports
                .entries
                .get_mut(&0)
                .unwrap()
                .1
                .missing
                .push(Port::BlockEntry(callee)),
            4 => reports
                .entries
                .get_mut(&0)
                .unwrap()
                .1
                .backedges
                .push(usize::MAX),
            5 => checker.invocations.get_mut(&site).unwrap().point = usize::MAX,
            6 => reports.effects.get_mut(&point).unwrap().0 = 1,
            7 => {
                let Effect::Call { function, .. } = &mut reports.effects.get_mut(&point).unwrap().1
                else {
                    panic!()
                };
                *function = usize::MAX;
            }
            _ => unreachable!(),
        }
        let counts = checker.edge_counts();
        let error = checker.call_graph(&reports, Span::default()).unwrap_err();
        assert_eq!(error.code, "B001", "fault {fault}");
        assert!(
            error.message.contains("identity mismatch"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn call_graph_bounds_nodes_sites_boundaries_and_work_atomically() {
    let (mut checker, reports) = checked("f<int32>:(){->1};x:f();y:f()");
    let expected = &reports.calls;
    let boundaries = expected
        .nodes
        .values()
        .map(|node| node.missing.len() + node.backedges.len())
        .sum::<usize>();
    assert!(boundaries > 0);
    for (nodes, sites, parts) in [
        (1, 2, boundaries),
        (2, 1, boundaries),
        (2, 2, boundaries - 1),
    ] {
        assert!(
            checker
                .call_graph_limited(&reports, Span::default(), nodes, sites, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    assert_eq!(
        &checker
            .call_graph_limited(&reports, Span::default(), 2, 2, boundaries)
            .unwrap(),
        expected
    );
    let before = checker.flow.work;
    checker.call_graph(&reports, Span::default()).unwrap();
    let work = checker.flow.work - before;
    let counts = checker.edge_counts();
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.call_graph(&reports, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(graph) = result {
            assert_eq!(&graph, expected);
        }
        assert_eq!(checker.edge_counts(), counts);
    }
}
