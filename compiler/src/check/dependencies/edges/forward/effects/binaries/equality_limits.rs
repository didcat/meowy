use super::{super::tests::checked, *};

#[test]
pub(crate) fn equality_effects_share_exact_work_and_map_limits_with_operand_effects() {
    for (missing, parts, pass) in [(0, 6, true), (0, 5, false), (1, 6, false)] {
        let (mut checker, mut reports) = checked("a:[1];b:[2];x:a==b", false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let expected = reports.effects.clone();
        let ops = checker.binaries.clone();
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
        assert_eq!(checker.binaries, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn equality_effects_keep_fixed_report_cost_at_the_capacity_boundary() {
    let mut costs = Vec::new();
    for capacity in [0, 1, crate::list::MAX_CAPACITY] {
        let source = format!("a<int32[{capacity}]>:[];b<int32[{capacity}]>:[];a==b");
        let (mut checker, mut reports) = checked(&source, false);
        let id = *checker.binaries.first_key_value().unwrap().0;
        reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Operation(id), Port::Normal(id)];
        let before = checker.flow.work;
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
            .unwrap();
        costs.push(checker.flow.work - before);
        let (_, Effect::Binary(observed)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(observed.types.inputs, [Class::List { capacity }; 2]);
        assert!(observed.operation && observed.result && observed.plan.equality);
    }
    assert!(costs.iter().all(|cost| *cost == costs[0]));
}

#[test]
pub(crate) fn equality_effects_merge_atomically_without_replacing_partial_observations() {
    let (mut checker, reports) = checked("a:[1];b:[2];a==b", false);
    let id = *checker.binaries.first_key_value().unwrap().0;
    let stage = checker
        .binary_effect_stage(&reports, 0, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    let result = checker
        .binary_effect_stage(&reports, 0, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_binary_effect(stage, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_binary_effect(stage, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..8 {
        let mut effects = expected.clone();
        let (owner, Effect::Binary(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.inputs.swap(0, 1),
            2 => op.types.inputs = [Class::List { capacity: 2 }; 2],
            3 => op.types.inputs = [Class::Record { fields: 1 }; 2],
            4 => op.plan.equality = false,
            5 => op.control = true,
            6 => op.op = "!=",
            7 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_binary_effect(result, &mut effects, 1, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
    }
    checker
        .record_binary_effect(stage, &mut effects, 1, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_binary_effect(result, &mut effects, 1, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(effects, expected);
}
