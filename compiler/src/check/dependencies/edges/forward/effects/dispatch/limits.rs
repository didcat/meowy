use super::{super::tests::checked, *};

#[test]
pub(crate) fn dispatch_effects_share_exact_work_and_map_limits_without_payload_copies() {
    let source = "n:1;v:n.{inner:2.{->$};->$};f<int32>:(p<int32>){->p.{->$}}";
    for missing in [0, 1] {
        let (mut checker, mut reports) = checked(source, false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let before = reports.effects.clone();
        let ops = checker.dispatch_ops.clone();
        let counts = checker.edge_counts();
        let limit = before.len() - missing;
        let start = checker.flow.work;
        let result = checker.operation_effects_limited(&reports, Span::default(), limit, 8, 0);
        assert_eq!(result.is_ok(), missing == 0);
        let work = checker.flow.work - start;
        if let Ok(actual) = result {
            assert_eq!(actual, before);
            for spare in [0, 1] {
                checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
                let result =
                    checker.operation_effects_limited(&reports, Span::default(), limit, 8, 0);
                assert_eq!(result.is_ok(), spare == 0);
                if let Ok(actual) = result {
                    assert_eq!(actual, before);
                }
            }
        }
        assert_eq!(reports.effects, before);
        assert_eq!(checker.dispatch_ops, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn dispatch_effects_keep_partial_reports_atomic_on_conflicts_and_exhaustion() {
    let (mut checker, _) = checked("v:3.{->$}", false);
    let id = *checker.dispatch_ops.first_key_value().unwrap().0;
    let mut effects = Effects::new();
    assert!(
        checker
            .record_dispatch_effect(0, Port::Operation(id), &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_dispatch_effect(0, Port::Operation(id), &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..7 {
        let mut effects = expected.clone();
        let (owner, Effect::Dispatch(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.input += 1,
            2 => op.local += 1,
            3 => op.block += 1,
            4 => op.normal = false,
            5 => op.control = true,
            6 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_dispatch_effect(0, Port::Normal(id), &mut effects, 1, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
    }
    checker
        .record_dispatch_effect(0, Port::Operation(id), &mut effects, 1, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    checker.dispatch_ops.get_mut(&id).unwrap().input_normal = false;
    assert!(
        checker
            .record_dispatch_effect(0, Port::Operation(id), &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
    checker.dispatch_ops.get_mut(&id).unwrap().input_normal = true;
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_dispatch_effect(0, Port::Normal(id), &mut effects, 1, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(effects, expected);
}

#[test]
pub(crate) fn dispatch_effects_keep_fixed_cost_for_large_receiver_values() {
    let mut costs = Vec::new();
    for len in [1, 65_536] {
        let source = format!("v:(\"{}\").{{->$}}", "x".repeat(len));
        let (mut checker, mut reports) = checked(&source, false);
        let id = *checker.dispatch_ops.first_key_value().unwrap().0;
        reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Operation(id), Port::Normal(id)];
        let before = checker.flow.work;
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
            .unwrap();
        costs.push(checker.flow.work - before);
        let (_, Effect::Dispatch(op)) = effects[&id] else {
            panic!()
        };
        assert!(op.initialized && op.result);
    }
    assert_eq!(costs[0], costs[1]);
}
