use super::{super::tests::checked, *};

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
