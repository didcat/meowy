use super::{super::tests::checked, *};

#[test]
pub(crate) fn temporary_effects_share_exact_work_and_effect_limits_without_payload_copies() {
    let source = "x:*(&7);y:*(&[1,2]);n:**(&(&8))";
    for missing in [0, 1] {
        let (mut checker, mut reports) = checked(source, false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let expected = reports.effects.clone();
        let ops = checker.temporary_borrows.clone();
        let counts = checker.edge_counts();
        let before = checker.flow.work;
        let limit = expected.len() - missing;
        let result = checker.operation_effects_limited(&reports, Span::default(), limit, 6, 0);
        assert_eq!(result.is_ok(), missing == 0);
        let work = checker.flow.work - before;
        if let Ok(actual) = result {
            assert_eq!(actual, expected);
            for spare in [0, 1] {
                checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
                let result =
                    checker.operation_effects_limited(&reports, Span::default(), limit, 6, 0);
                assert_eq!(result.is_ok(), spare == 0);
                if let Ok(actual) = result {
                    assert_eq!(actual, expected);
                }
            }
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.temporary_borrows, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn temporary_effects_keep_partial_records_on_conflicts_and_limits() {
    let (mut checker, _) = checked("x:*(&7)", false);
    let id = *checker.temporary_borrows.first_key_value().unwrap().0;
    let mut effects = Effects::new();
    assert!(
        checker
            .record_temporary_effect(0, Port::Operation(id), &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_temporary_effect(0, Port::Operation(id), &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..6 {
        let mut effects = expected.clone();
        let (owner, Effect::Temporary(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.input += 1,
            2 => op.local += 1,
            3 => op.statement += 1,
            4 => op.control = true,
            5 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_temporary_effect(0, Port::Normal(id), &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    for port in [Port::Entry(id), Port::Operation(usize::MAX)] {
        assert!(
            checker
                .record_temporary_effect(0, port, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, expected);
    }
    checker
        .record_temporary_effect(0, Port::Operation(id), &mut effects, 1, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_temporary_effect(0, Port::Normal(id), &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}

#[test]
pub(crate) fn temporary_effects_keep_fixed_cost_for_large_initializer_values() {
    let mut costs = Vec::new();
    for len in [1, 65_536] {
        let source = format!("x:*(&\"{}\")", "x".repeat(len));
        let (mut checker, mut reports) = checked(&source, false);
        let id = *checker.temporary_borrows.first_key_value().unwrap().0;
        reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Operation(id), Port::Normal(id)];
        let before = checker.flow.work;
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
            .unwrap();
        let (_, Effect::Temporary(op)) = &effects[&id] else {
            panic!()
        };
        assert!(op.acquired && op.result);
        costs.push(checker.flow.work - before);
    }
    assert_eq!(costs[0], costs[1]);
}
