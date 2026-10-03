use super::{super::tests::checked, *};

#[test]
pub(crate) fn unchanged_narrowing_inputs_reject_header_and_stage_conflicts_atomically() {
    for fault in 0..13 {
        let (mut checker, mut reports) = checked("v:1;x:v", false);
        let id = *checker.narrowings.first_key_value().unwrap().0;
        let (owner, Effect::Narrowing(observed)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        let op = checker.narrowings.get_mut(&id).unwrap();
        match fault {
            0 => *owner = 9,
            1 => op.owner = 9,
            2 => observed.input = id,
            3 => observed.changed = true,
            4 => observed.normal = false,
            5 => observed.control = !observed.control,
            6 => observed.operation = true,
            7 => {
                observed.result = false;
                observed.control = !observed.control;
            }
            8 => {
                checker.narrowings.remove(&id);
            }
            9 => op.edges.clear(),
            10 => op.edges[1].route = Route::Returned,
            11 => {
                op.changed = true;
                observed.changed = true;
                observed.input = id;
            }
            12 => {
                op.normal = false;
                observed.normal = false;
                observed.input = id;
            }
            _ => unreachable!(),
        }
        let before = reports.effects.clone();
        let parts = reports.parts;
        let ops = checker.narrowings.clone();
        let counts = checker.edge_counts();
        let error = checker
            .unchanged_narrowing_input(&reports, id, 0, Span::default())
            .unwrap_err();
        assert!(
            error.message.contains("narrowing-effect identity"),
            "fault {fault}"
        );
        assert_eq!(reports.effects, before);
        assert_eq!(reports.parts, parts);
        assert_eq!(checker.narrowings, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn unchanged_narrowing_inputs_bound_observed_and_opaque_lookup_work() {
    for state in 0..3 {
        let (mut checker, mut reports) = checked("v:1;x:v", false);
        let (&id, op) = checker.narrowings.first_key_value().unwrap();
        let input = op.input;
        if state == 1 {
            let (_, Effect::Narrowing(observed)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            observed.result = false;
        } else if state == 2 {
            reports.effects.remove(&id);
        }
        let before = reports.effects.clone();
        let parts = reports.parts;
        let ops = checker.narrowings.clone();
        let counts = checker.edge_counts();
        let start = checker.flow.work;
        let expected = (state == 0).then_some(input);
        assert_eq!(
            checker
                .unchanged_narrowing_input(&reports, id, 0, Span::default())
                .unwrap(),
            expected
        );
        let work = checker.flow.work - start;
        for spare in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
            let result = checker.unchanged_narrowing_input(&reports, id, 0, Span::default());
            if spare == 0 {
                assert_eq!(result.unwrap(), expected);
            } else {
                assert!(
                    result
                        .unwrap_err()
                        .message
                        .contains("narrowing-effect budget")
                );
            }
            assert_eq!(reports.effects, before);
            assert_eq!(reports.parts, parts);
            assert_eq!(checker.narrowings, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}
