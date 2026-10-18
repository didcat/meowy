use super::{super::tests::checked, *};

#[test]
pub(crate) fn scalar_literal_reports_keep_values_with_independent_operation_and_result_visits() {
    let (mut checker, mut reports) = checked("((7))", false);
    let id = *checker.scalar_leaves.first_key_value().unwrap().0;
    for (port, operation, result) in [
        (Port::Operation(id), true, false),
        (Port::Normal(id), false, true),
    ] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects(&reports, Span::default())
            .unwrap();
        let (_, Effect::Scalar(leaf)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(leaf.value, Some(ScalarLiteral::Int(7)));
        assert_eq!((leaf.operation, leaf.result), (operation, result));
    }
}

#[test]
pub(crate) fn scalar_literal_reports_reject_missing_mismatched_and_out_of_range_values() {
    for value in [
        None,
        Some(ScalarLiteral::Null),
        Some(ScalarLiteral::Bool(false)),
        Some(ScalarLiteral::Int(256)),
    ] {
        let (mut checker, reports) = checked("n<uint8>:7", false);
        let id = *checker.scalar_leaves.first_key_value().unwrap().0;
        checker.scalar_leaves.get_mut(&id).unwrap().value = value;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .operation_effects(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn scalar_literal_reports_reject_stale_values_before_new_or_partial_publication() {
    let (mut checker, reports) = checked("7", false);
    let id = *checker.scalar_leaves.first_key_value().unwrap().0;
    let stage = checker
        .scalar_effect_stage(&reports, 0, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    for fresh in [false, true] {
        let mut effects = if fresh {
            Effects::new()
        } else {
            reports.effects.clone()
        };
        let before = effects.clone();
        let mut changed = stage;
        changed.value = Some(ScalarLiteral::Int(8));
        assert!(
            checker
                .record_scalar_effect(changed, &mut effects, MAX_EDGES, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
    }
}
