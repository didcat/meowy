use super::{super::super::tests::checked, *};

#[test]
pub(crate) fn candidate_graph_preserves_borrowed_histories_positions_and_source_descriptors() {
    let (mut checker, reports) =
        checked("flag:=false;r:{|flag|->n:1;|!flag|->n:2};s:{->r};u:{->n:=3};f<int32>:(){->4}");
    let before = format!("{reports:?}");
    let (graph, parts) = checker.candidate_graph(&reports, Span::default()).unwrap();
    assert!(std::ptr::eq(graph.results, &reports.results));
    assert!(std::ptr::eq(graph.inputs, &reports.candidate_inputs));
    assert!(parts < reports.parts);
    assert!(graph.results.values().any(|(_, result)| {
        result
            .slots
            .as_ref()
            .is_some_and(|slots| slots.contains(&Sources::Unknown))
    }));
    assert!(
        graph
            .inputs
            .values()
            .any(|(_, input)| input.source.is_some())
    );
    assert!(graph.inputs.values().any(|(owner, _)| *owner != 0));
    assert_eq!(format!("{reports:?}"), before);
}

#[test]
pub(crate) fn candidate_graph_rejects_missing_extra_and_corrupt_stored_inputs() {
    for fault in 0..10 {
        let (mut checker, mut reports) = checked("r:{->7;->n:1};s:{->r}");
        let key = *reports.candidate_inputs.last_key_value().unwrap().0;
        let (owner, input) = reports.candidate_inputs.get_mut(&key).unwrap();
        match fault {
            0 => *owner += 1,
            1 => input.point = usize::MAX,
            2 => input.candidate.emission = usize::MAX,
            3 => input.candidate.statement = usize::MAX,
            4 => input.candidate.target += 1,
            5 => input.projection = Projection::Value,
            6 => input.source = None,
            7 => input.source.as_mut().unwrap().block = key.0,
            8 => {
                reports.candidate_inputs.remove(&key);
            }
            9 => {
                let value = reports.candidate_inputs[&key];
                reports
                    .candidate_inputs
                    .insert((key.0, key.1, usize::MAX), value);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let result = checker.candidate_graph(&reports, Span::default());
        assert!(result.is_err(), "fault {fault}");
        assert!(result.err().unwrap().message.contains("identity"));
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn candidate_graph_bounds_qualification_scratch_and_work_exactly() {
    let (mut checker, reports) = checked("r:{->7;->n:1};s:{->r}");
    let before = format!("{reports:?}");
    let work = checker.flow.work;
    let (_, parts) = checker
        .candidate_graph_limited(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - work;
    let scratch = MAX_EDGES - parts;
    assert_eq!(
        checker
            .candidate_graph_limited(&reports, Span::default(), scratch)
            .unwrap()
            .1,
        0
    );
    assert!(
        checker
            .candidate_graph_limited(&reports, Span::default(), scratch - 1)
            .is_err()
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.candidate_graph_limited(&reports, Span::default(), MAX_EDGES);
        assert_eq!(result.is_ok(), short == 0);
        assert_eq!(format!("{reports:?}"), before);
    }
}

#[test]
pub(crate) fn candidate_graph_rejects_empty_histories_for_mutable_and_aggregate_slots() {
    for source in ["r:{->n:=1}", "r:{->inner:{->n:1}}"] {
        let (mut checker, mut reports) = checked(source);
        let slot = reports
            .results
            .values_mut()
            .filter_map(|(_, result)| result.slots.as_mut())
            .flatten()
            .find(|source| **source == Sources::Unknown)
            .unwrap();
        *slot = Sources::Candidates(Vec::new());
        assert!(checker.candidate_graph(&reports, Span::default()).is_err());
    }
}
