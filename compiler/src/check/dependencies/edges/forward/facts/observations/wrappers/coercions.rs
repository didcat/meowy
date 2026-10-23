use super::super::tests::context;
use super::*;
use crate::check::dependencies::CoercionKind;

#[test]
pub(crate) fn scalar_forward_coercions_preserve_exact_roots_and_exclude_conversions_and_primaries()
{
    let (mut checker, reports, ports) = context(
        "a<uint8>:((7));b<int32><null>:7;c<int32>:{->7};d:@\"debug\";stop<int32>:d.panic(\"stop\")",
    );
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observed, _) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    let mut count = 0;
    for (&point, op) in &checker.coercions {
        let eligible = op.kind == CoercionKind::Forward
            && !op.primary
            && graph.nodes.contains_key(&Port::Normal(point));
        assert_eq!(
            observed.forwarded.get(&Port::Normal(point)),
            eligible.then_some(&Forward {
                owner: op.owner,
                input: Port::Normal(op.input),
                control: op.control
            })
        );
        assert!(!observed.forwarded.contains_key(&Port::Operation(point)));
        count += usize::from(eligible);
    }
    assert!(count >= 3);
}

#[test]
pub(crate) fn scalar_forward_coercions_reject_stale_stages_controls_and_predecessors() {
    for fault in 0..6 {
        let (mut checker, mut reports, ports) = context("n<int32>:(7)");
        let point = *checker.coercions.first_key_value().unwrap().0;
        if fault < 3 {
            let (_, Effect::Coercion(op)) = reports.effects.get_mut(&point).unwrap() else {
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
            4 => checker.coercions.get_mut(&point).unwrap().input = point,
            5 => graph.nodes.get_mut(&Port::Normal(point)).unwrap().incoming[0].edge = usize::MAX,
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
pub(crate) fn scalar_forward_coercions_share_group_work_and_payload_limits() {
    let (mut checker, reports, ports) = context("n<uint64>:((7))");
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
}
