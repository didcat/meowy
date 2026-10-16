use super::*;
use crate::check::dependencies::edges::forward::results::tests::checked;

#[test]
pub(crate) fn argument_graph_borrows_original_sources_and_candidate_maps() {
    let (mut checker, reports) =
        checked("f:(a<int32>,b<int32>,c<int32>,d<int32>){};r:{->n:1};f(r.n,({->r.n}),3.{->$},4)");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (graph, sources, parts) = checker
        .argument_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(std::ptr::eq(sources, &reports.argument_sources));
    assert!(std::ptr::eq(graph.graph.results, &reports.results));
    assert!(std::ptr::eq(graph.graph.inputs, &reports.candidate_inputs));
    assert!(std::ptr::eq(graph.sources, &reports.direct_sources));
    assert!(parts < MAX_EDGES);
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn argument_graph_rejects_stale_argument_candidate_and_direct_maps_atomically() {
    for fault in 0..6 {
        let (mut checker, mut reports) =
            checked("f:(a<int32>,b<int32>){};r:{->n:1};f(2,({->r.n}))");
        let point = checker.invocations.first_key_value().unwrap().1.args[1];
        let source = reports.argument_sources[&point].1.block.unwrap();
        let key = *reports
            .candidate_inputs
            .keys()
            .find(|key| key.0 == source.slot.block)
            .unwrap();
        match fault {
            0 => {
                reports.argument_sources.remove(&point);
            }
            1 => reports.argument_sources.get_mut(&point).unwrap().1.point = usize::MAX,
            2 => reports.candidate_inputs.get_mut(&key).unwrap().1.point = usize::MAX,
            3 => {
                reports.direct_sources.remove(&key);
            }
            4 => reports.results.get_mut(&source.slot.block).unwrap().0 += 1,
            5 => reports.arguments.get_mut(&point).unwrap().param = usize::MAX,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let Err(error) = checker.argument_graph(&reports, Span::default(), MAX_EDGES) else {
            panic!("fault {fault}");
        };
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_graph_shares_exact_qualification_work_and_cache_budgets() {
    let (mut checker, reports) = checked("f:(a<int32>,b<int32>){};r:{->n:1};s:{->r.n};f(r.n,s)");
    let start = checker.flow.work;
    let (_, _, left) = checker
        .argument_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - start;
    let used = MAX_EDGES - left;
    assert!(used > 0);
    let start = checker.flow.work;
    let (_, parts) = checker
        .argument_source_view(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (_, parts) = checker
        .direct_graph(&reports, Span::default(), parts)
        .unwrap();
    assert_eq!(parts, left);
    assert_eq!(checker.flow.work - start, work);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for parts in [used - 1, used] {
        let result = checker.argument_graph(&reports, Span::default(), parts);
        if parts == used {
            assert_eq!(result.unwrap().2, 0);
        } else {
            assert!(result.err().unwrap().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.argument_graph(&reports, Span::default(), used);
        if short == 0 {
            assert_eq!(result.unwrap().2, 0);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.err().unwrap().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
