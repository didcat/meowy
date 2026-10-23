use super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_narrowing_observations_preserve_unchanged_inputs_and_exclude_changed_types() {
    let (mut checker, reports, ports) =
        context("f:(v<int32><null>){v;|v<int32>|{v}};g<never>:(n<never>){->n}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observed, _) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    let mut changed = 0;
    for (&point, op) in &checker.narrowings {
        let eligible = !op.changed && op.normal && graph.nodes.contains_key(&Port::Normal(point));
        assert_eq!(
            observed.forwarded.get(&Port::Normal(point)),
            eligible.then_some(&Forward {
                owner: op.owner,
                input: Port::Normal(op.input),
                control: op.control
            })
        );
        assert!(!observed.forwarded.contains_key(&Port::Operation(point)));
        changed += usize::from(op.changed);
    }
    assert_eq!(changed, 1);
}

#[test]
pub(crate) fn scalar_narrowing_observations_reject_missing_results_and_mismatched_metadata() {
    for fault in 0..6 {
        let (mut checker, mut reports, ports) = context("f:(n<int32>){((n))}");
        let point = *checker.narrowings.first_key_value().unwrap().0;
        if fault < 3 {
            let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&point).unwrap() else {
                panic!()
            };
            match fault {
                0 => op.result = false,
                1 => op.operation = true,
                2 => op.control = !op.control,
                _ => unreachable!(),
            }
        }
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        match fault {
            3 => {
                graph.nodes.remove(&Port::Normal(point));
            }
            4 => checker.narrowings.get_mut(&point).unwrap().input = point,
            5 => {
                graph.nodes.get_mut(&Port::Normal(point)).unwrap().incoming[0]
                    .source
                    .block = usize::MAX
            }
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
        assert_eq!(format!("{reports:?}{graph:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_narrowing_observations_share_wrapper_budgets_and_preserve_controls() {
    let (mut checker, mut reports, ports) = context("f:(n<int32>){v<int32>:((n))}");
    let point = *checker.narrowings.first_key_value().unwrap().0;
    checker.narrowings.get_mut(&point).unwrap().control = true;
    let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&point).unwrap() else {
        panic!()
    };
    op.control = true;
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
    assert!(expected.forwarded[&Port::Normal(point)].control);
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
}
