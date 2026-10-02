use super::{super::tests::checked, *};

#[test]
pub(crate) fn exclusive_effects_share_exact_work_effect_and_payload_limits() {
    let source = "r:{->rows:=[{->xs:=[1]}]};p:&!(r.rows[1].xs[1])";
    for (missing, parts, pass) in [(0, 21, true), (0, 20, false), (1, 21, false)] {
        let (mut checker, mut reports) = checked(source, false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let expected = reports.effects.clone();
        let ops = checker.exclusives.clone();
        let counts = checker.edge_counts();
        let limit = expected.len() - missing;
        let before = checker.flow.work;
        let result = checker.operation_effects_limited(&reports, Span::default(), limit, parts, 0);
        assert_eq!(result.is_ok(), pass);
        let work = checker.flow.work - before;
        if let Ok(actual) = result {
            assert_eq!(actual, expected);
            for spare in [0, 1] {
                checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
                let result =
                    checker.operation_effects_limited(&reports, Span::default(), limit, parts, 0);
                assert_eq!(result.is_ok(), spare == 0);
                if let Ok(actual) = result {
                    assert_eq!(actual, expected);
                }
            }
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.exclusives, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn exclusive_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, _) = checked("r:{->rows:=[{->xs:=[1]}]};p:&!(r.rows[1].xs[1])", false);
    let id = *checker.exclusives.first_key_value().unwrap().0;
    let reserve = Port::Reserve { point: id, step: 0 };
    let mut effects = Effects::new();
    let mut parts = 14;
    assert!(
        checker
            .record_exclusive_effect(0, reserve, &mut effects, 1, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 14);
    parts = 15;
    assert!(
        checker
            .record_exclusive_effect(0, reserve, &mut effects, 0, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 15);
    checker
        .record_exclusive_effect(0, reserve, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(parts, 0);
    let expected = effects.clone();
    for fault in 0..14 {
        let mut effects = expected.clone();
        let (owner, Effect::Exclusive(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.place.root += 1,
            2 => op.place.fields[0] += 1,
            3 => op.storage += 1,
            4 => op.steps[1] = PathStep::Field(1),
            5 => op.counts[0] += 1,
            6 => op.access[0].length = Some(1),
            7 => op.access[0].normal = false,
            8 => op.normal = false,
            9 => op.control = true,
            10 => op.addresses.clear(),
            11 => op.reservations.clear(),
            12 => {
                let PathStep::Index { point, .. } = &mut op.steps[0] else {
                    panic!()
                };
                *point += 1;
            }
            13 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_exclusive_effect(
                    0,
                    Port::Normal(id),
                    &mut effects,
                    1,
                    &mut parts,
                    Span::default()
                )
                .is_err()
        );
        assert_eq!(effects, before);
        assert_eq!(parts, 0);
    }
    for port in [
        Port::Entry(id),
        Port::Reserve { point: id, step: 1 },
        Port::Address { point: id, step: 4 },
    ] {
        assert!(
            checker
                .record_exclusive_effect(0, port, &mut effects, 1, &mut parts, Span::default())
                .is_err()
        );
        assert_eq!(effects, expected);
    }
    checker
        .record_exclusive_effect(0, reserve, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    assert_eq!(parts, 0);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_exclusive_effect(
                0,
                Port::Normal(id),
                &mut effects,
                1,
                &mut parts,
                Span::default()
            )
            .is_err()
    );
    assert_eq!(effects, expected);
    assert_eq!(parts, 0);
}

#[test]
pub(crate) fn exclusive_effects_bound_combined_prefix_and_path_payloads() {
    let (mut checker, mut reports) = checked("r:{->xs:=[1]};p:&!(r.xs[1])", false);
    let id = *checker.exclusives.first_key_value().unwrap().0;
    let op = checker.exclusives.get_mut(&id).unwrap();
    let max = crate::list::MAX_WRITE_PATH;
    op.place.fields = vec![0; max - 1];
    op.counts = vec![1; max - 1];
    let reserve = Port::Reserve { point: id, step: 0 };
    reports.entries.get_mut(&0).unwrap().1.ports = vec![reserve, reserve];
    let before = reports.effects.clone();
    let size = max * 2 + 3;
    let effects = checker
        .operation_effects_limited(&reports, Span::default(), 1, size, 0)
        .unwrap();
    let (_, Effect::Exclusive(op)) = &effects[&id] else {
        panic!()
    };
    assert_eq!(op.place.fields.len() + op.steps.len(), max);
    assert_eq!(op.addresses, [false, false]);
    assert_eq!(op.reservations, [true]);
    assert!(!op.acquired && !op.result);
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 1, size - 1, 0)
            .is_err()
    );
    let op = checker.exclusives.get_mut(&id).unwrap();
    op.place.fields.push(0);
    op.counts.push(1);
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 1, usize::MAX, 0)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(reports.effects, before);
}
