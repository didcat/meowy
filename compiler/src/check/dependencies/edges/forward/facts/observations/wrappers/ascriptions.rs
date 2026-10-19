use super::super::tests::context;
use super::*;
use crate::check::dependencies::TypedKind;

#[test]
pub(crate) fn scalar_ascription_observations_keep_operation_and_result_stages_independent() {
    for (operation, result) in [(true, false), (false, true), (true, true)] {
        let (mut checker, mut reports, ports) = context("(7)~<int32>");
        let point = *checker.typed_ops.first_key_value().unwrap().0;
        let (_, Effect::Typed(op)) = reports.effects.get_mut(&point).unwrap() else {
            panic!()
        };
        op.operation = operation;
        op.result = result;
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        if !operation {
            let node = graph.nodes.remove(&Port::Operation(point)).unwrap();
            graph.external.insert(Port::Operation(point), node.site);
            graph.nodes.get_mut(&Port::Normal(point)).unwrap().incoming[0].reached = false;
        }
        if !result {
            graph.nodes.remove(&Port::Normal(point));
        }
        let definitions = definitions::Definitions::default();
        let (observed, _) = checker
            .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(
            observed.forwarded.contains_key(&Port::Operation(point)),
            operation
        );
        assert_eq!(
            observed.forwarded.contains_key(&Port::Normal(point)),
            result
        );
    }
}

#[test]
pub(crate) fn scalar_ascription_observations_exclude_predicates_changed_types_and_stopped_inputs() {
    let (mut checker, reports, ports) =
        context("7<int32>;<U>:<int32><null>;7~<U>;d:@\"debug\";d.panic(\"stop\")~<int32>");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observed, _) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(
        checker
            .typed_ops
            .values()
            .any(|op| op.kind == TypedKind::Predicate)
    );
    assert!(checker.typed_ops.values().any(|op| op.changed));
    assert!(checker.typed_ops.values().any(|op| !op.normal));
    for point in checker.typed_ops.keys() {
        assert!(!observed.forwarded.contains_key(&Port::Operation(*point)));
        assert!(!observed.forwarded.contains_key(&Port::Normal(*point)));
    }
}

#[test]
pub(crate) fn scalar_ascription_observations_reject_stale_stages_and_bound_shared_work() {
    let (mut checker, mut reports, ports) = context("(7)~<int32>");
    let point = *checker.typed_ops.first_key_value().unwrap().0;
    let (graph, _) = checker
        .scalar_graph(&reports, ports.clone(), Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let start = checker.flow.work;
    let (_, left) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        assert_eq!(
            checker
                .scalar_observations(&reports, &graph, &definitions, Span::default(), room)
                .is_ok(),
            pass
        );
    }
    checker.flow.work = 0;
    checker.flow.full = false;
    drop(graph);
    let (_, Effect::Typed(op)) = reports.effects.get_mut(&point).unwrap() else {
        panic!()
    };
    op.operation = false;
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(
        checker
            .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
            .is_err()
    );
}
