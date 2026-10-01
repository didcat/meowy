use super::{super::tests::checked, *};

#[test]
pub(crate) fn reborrow_effects_share_exact_work_and_effect_limits_with_place_borrows() {
    let source = "n:=1;p:&!n;q:&!*p;r<&int32>:q";
    for missing in [0, 1] {
        let (mut checker, mut reports) = checked(source, false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let expected = reports.effects.clone();
        let ops = checker.reborrow_ops.clone();
        let counts = checker.edge_counts();
        let limit = expected.len() - missing;
        let before = checker.flow.work;
        let result = checker.operation_effects_limited(&reports, Span::default(), limit, 1, 0);
        assert_eq!(result.is_ok(), missing == 0);
        let work = checker.flow.work - before;
        if let Ok(actual) = result {
            assert_eq!(actual, expected);
            for spare in [0, 1] {
                checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
                let result =
                    checker.operation_effects_limited(&reports, Span::default(), limit, 1, 0);
                assert_eq!(result.is_ok(), spare == 0);
                if let Ok(actual) = result {
                    assert_eq!(actual, expected);
                }
            }
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.reborrow_ops, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn reborrow_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, _) = checked("n:=1;p:&!n;q:&!*p", false);
    let id = *checker.reborrow_ops.first_key_value().unwrap().0;
    let mut effects = Effects::new();
    assert!(
        checker
            .record_reborrow_effect(0, Port::Operation(id), &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_reborrow_effect(0, Port::Operation(id), &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..7 {
        let mut effects = expected.clone();
        let (owner, Effect::Reborrow(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.parent += 1,
            2 => op.site += 1,
            3 => op.parent_mode = ReferenceMode::Shared,
            4 => op.mode = ReferenceMode::Shared,
            5 => op.control = true,
            6 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_reborrow_effect(0, Port::Normal(id), &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    for port in [Port::Entry(id), Port::Operation(usize::MAX)] {
        assert!(
            checker
                .record_reborrow_effect(0, port, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, expected);
    }
    checker
        .record_reborrow_effect(0, Port::Operation(id), &mut effects, 1, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_reborrow_effect(0, Port::Normal(id), &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}

#[test]
pub(crate) fn reborrow_effects_keep_fixed_record_cost_for_large_referent_shapes() {
    let mut costs = Vec::new();
    for len in [1, 128] {
        let fields = (0..len).map(|id| format!("->n{id}:1;")).collect::<String>();
        let (mut checker, reports) = checked(&format!("r:{{{fields}}};p:&r;q:&*p"), false);
        let id = *checker.reborrow_ops.first_key_value().unwrap().0;
        let mut effects = Effects::new();
        let before = checker.flow.work;
        checker
            .record_reborrow_effect(0, Port::Operation(id), &mut effects, 1, Span::default())
            .unwrap();
        checker
            .record_reborrow_effect(0, Port::Normal(id), &mut effects, 1, Span::default())
            .unwrap();
        assert_eq!(effects[&id], reports.effects[&id]);
        costs.push(checker.flow.work - before);
    }
    assert_eq!(costs[0], costs[1]);
}
