use super::{
    super::{super::tests::checked, tests::keys},
    *,
};

#[test]
pub(crate) fn candidate_inputs_collect_original_positions_without_selecting_histories() {
    let (mut checker, reports) =
        checked("flag:=false;r:{|flag|->n:1;|!flag|->n:2};s:{->r};f<int32>:(){->3}");
    assert_eq!(
        reports.candidate_inputs.keys().copied().collect::<Vec<_>>(),
        keys(&reports)
    );
    let mut multiple = false;
    for (&(block, slot, position), &(owner, input)) in &reports.candidate_inputs {
        let (seen, result) = &reports.results[&block];
        assert_eq!(owner, *seen);
        let Sources::Candidates(values) = &result.slots.as_ref().unwrap()[slot] else {
            panic!()
        };
        assert_eq!(input.candidate, values[position]);
        multiple |= values.len() == 2;
    }
    assert!(multiple);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (inputs, _) = checker
        .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert_eq!(inputs, reports.candidate_inputs);
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn candidate_inputs_preserve_unknown_empty_and_stopped_result_reports() {
    for source in [
        "v:{};r:{->n:=1;->xs:[1,2];->inner:{->n:2}}",
        "f<int32><null>:(flag<boolean>){|flag|->1}",
        "d:@\"debug\";v:{->n:1;d.panic(\"stop\")}",
    ] {
        let (checker, reports) = checked(source);
        assert_eq!(
            reports.candidate_inputs.keys().copied().collect::<Vec<_>>(),
            keys(&reports)
        );
        for (&block, (_, result)) in &reports.results {
            let Some(slots) = &result.slots else {
                assert!(reports.candidate_inputs.keys().all(|key| key.0 != block));
                continue;
            };
            for (slot, sources) in slots.iter().enumerate() {
                if matches!(sources, Sources::Unknown)
                    || matches!(sources, Sources::Candidates(values) if values.is_empty())
                {
                    assert!(
                        reports
                            .candidate_inputs
                            .keys()
                            .all(|key| (key.0, key.1) != (block, slot))
                    );
                }
            }
        }
        for id in checker.bodies.keys() {
            if !reports.results.contains_key(id) {
                assert!(reports.candidate_inputs.keys().all(|key| key.0 != *id));
            }
        }
    }
}
