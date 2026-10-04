use super::{super::tests::checked, *};

#[test]
pub(crate) fn field_observations_preserve_partial_reports_on_conflicts_and_limits() {
    for source in ["r:{->n:1};x:r.n", "r:{->n:1};p:&r;x:p.n"] {
        let (mut checker, reports) = checked(source, false);
        let id = *checker.fields.first_key_value().unwrap().0;
        let operation = checker
            .field_effect_stage(&reports, 0, Port::Operation(id), Span::default())
            .unwrap()
            .unwrap();
        let result = checker
            .field_effect_stage(&reports, 0, Port::Normal(id), Span::default())
            .unwrap()
            .unwrap();
        let mut effects = Effects::new();
        assert!(
            checker
                .record_field_effect(result, &mut effects, 0, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(effects.is_empty());
        checker
            .record_field_effect(result, &mut effects, 1, Span::default())
            .unwrap();
        let expected = effects.clone();
        for fault in 0..7 {
            let mut effects = expected.clone();
            let mut stage = operation;
            match fault {
                0 => stage.owner += 1,
                1 => stage.input = usize::MAX,
                2 => stage.index += 1,
                3 => stage.load = !stage.load,
                4 => stage.normal = false,
                5 => stage.control = !stage.control,
                6 => {
                    effects.insert(id, (0, Effect::Unknown));
                }
                _ => unreachable!(),
            }
            let before = effects.clone();
            assert!(
                checker
                    .record_field_effect(stage, &mut effects, 1, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(effects, before);
        }
        checker
            .record_field_effect(operation, &mut effects, 1, Span::default())
            .unwrap();
        let expected = effects.clone();
        for stage in [operation, result] {
            checker
                .record_field_effect(stage, &mut effects, 1, Span::default())
                .unwrap();
        }
        assert_eq!(effects, expected);
        checker.flow.work = crate::flow::MAX_PROOF_WORK;
        assert!(
            checker
                .record_field_effect(operation, &mut effects, 1, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(effects, expected);
    }
}

#[test]
pub(crate) fn field_observations_bound_sparse_results_without_payload_or_partial_maps() {
    let (mut checker, mut reports) = checked("a:{->n:1}.n;b:{->n:2}.n", false);
    let ids: Vec<_> = checker.fields.keys().copied().collect();
    reports.entries.get_mut(&0).unwrap().1.ports = vec![
        Port::Normal(ids[0]),
        Port::Operation(ids[1]),
        Port::Normal(ids[0]),
    ];
    let before = format!("{reports:?}{:?}{:?}", checker.fields, checker.edge_counts());
    let work = checker.flow.work;
    let (effects, parts) = checker
        .operation_effects_with_room(&reports, Span::default(), 2, 0, 0)
        .unwrap();
    let work = checker.flow.work - work;
    assert_eq!(effects.len(), 2);
    assert_eq!(parts, 0);
    assert!(
        checker
            .operation_effects_with_room(&reports, Span::default(), 1, 0, 0)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.operation_effects_with_room(&reports, Span::default(), 2, 0, 0);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok((found, remaining)) = result {
            assert_eq!(found, effects);
            assert_eq!(remaining, 0);
        }
        assert_eq!(
            format!("{reports:?}{:?}{:?}", checker.fields, checker.edge_counts()),
            before
        );
    }
}

#[test]
pub(crate) fn field_observations_discard_late_result_identity_failures() {
    for fault in 0..3 {
        let (mut checker, mut reports) = checked("a:{->n:1}.n;b:{->n:2}.n", false);
        let ids: Vec<_> = checker.fields.keys().copied().collect();
        reports.entries.get_mut(&0).unwrap().1.ports =
            vec![Port::Operation(ids[0]), Port::Normal(ids[1])];
        let id = ids[1];
        match fault {
            0 => checker.fields.get_mut(&id).unwrap().owner += 1,
            1 => checker
                .fields
                .get_mut(&id)
                .unwrap()
                .edges
                .pop()
                .map(|_| ())
                .unwrap(),
            2 => {
                reports.index.operations.remove(&id);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}{:?}", checker.fields, checker.edge_counts());
        let error = checker
            .operation_effects(&reports, Span::default())
            .unwrap_err();
        assert!(error.message.contains("field-effect identity"));
        assert_eq!(
            format!("{reports:?}{:?}{:?}", checker.fields, checker.edge_counts()),
            before
        );
    }
}
