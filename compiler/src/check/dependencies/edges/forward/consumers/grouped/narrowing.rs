use super::{super::tests::checked, *};

#[test]
pub(crate) fn narrowing_consumers_traverse_local_wrappers_without_initializer_joins() {
    for source in [
        "r:{->n:1};s<{n<int32>}>:((r));v:s.n",
        "f<int32>:(){r:{->n:1};s<{n<int32>}>:((r));->s.n}",
    ] {
        let (mut checker, reports) = checked(source);
        let root = checker
            .operations
            .values()
            .filter_map(|op| op.input)
            .find(|id| {
                checker
                    .coercions
                    .get(id)
                    .is_some_and(|op| checker.group_inputs.contains_key(&op.input))
            })
            .unwrap();
        let owner = checker.points[root].owner;
        let mut raw = root;
        for index in 0..6 {
            raw = if index == 5 {
                checker.narrowings[&raw].input
            } else if index % 2 == 0 {
                checker.coercions[&raw].input
            } else {
                checker.group_inputs[&raw].input
            };
        }
        assert!(matches!(reports.effects[&raw].1, Effect::Read { .. }));
        assert!(
            checker
                .grouped_consumer_limited(&reports, root, owner, Span::default(), 6)
                .unwrap()
                .is_none()
        );
        let error = checker
            .grouped_consumer_limited(&reports, root, owner, Span::default(), 5)
            .unwrap_err();
        assert!(error.message.contains("budget"));
        assert!(reports.slot_uses.is_empty());
    }
}

#[test]
pub(crate) fn narrowing_consumers_keep_field_values_separate_from_field_inputs() {
    let (mut checker, reports) = checked("v:(({->inner:{->n:1}}).inner).n");
    assert_eq!(reports.slot_uses.len(), 1);
    let ids = checker
        .narrowings
        .iter()
        .map(|(&id, op)| (id, op.owner, op.input))
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    for (id, owner, input) in ids {
        assert!(checker.fields.contains_key(&input));
        assert_eq!(
            checker
                .unchanged_narrowing_input(&reports, id, owner, Span::default())
                .unwrap(),
            Some(input)
        );
        assert!(
            checker
                .grouped_consumer(&reports, id, owner, Span::default())
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
}
