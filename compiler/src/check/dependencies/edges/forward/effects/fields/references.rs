use super::{super::tests::checked, *};

#[test]
pub(crate) fn shared_field_reports_keep_source_types_at_independent_observations() {
    let source = "n:1;p:&n;x:p.{->$;->tag:true}.{->$.tag};f<boolean>:(p<&uint8>){->{->p;->tag:true}.{->$.tag}}";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, false);
    let fields: Vec<_> = checker
        .fields
        .iter()
        .map(|(&id, op)| (id, op.clone()))
        .collect();
    assert_eq!(fields.len(), 2);
    for (id, op) in fields {
        for port in [Port::Operation(id), Port::Normal(id)] {
            let stage = checker
                .field_effect_stage(&reports, op.owner, port, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(stage.shared_primary, op.shared_primary);
            assert!(stage.shared_primary.is_some());
            let mut effects = Effects::new();
            for _ in 0..2 {
                checker
                    .record_field_effect(stage, &mut effects, 1, Span::default())
                    .unwrap();
            }
            let effect = &effects[&id].1;
            let Effect::Field {
                shared_primary,
                operation,
                result,
                ..
            } = effect
            else {
                panic!()
            };
            assert_eq!(*shared_primary, op.shared_primary);
            assert_eq!(
                (*operation, *result),
                (port == Port::Operation(id), port == Port::Normal(id))
            );
            checker
                .validate_field_report(&reports, id, op.owner, effect, Span::default())
                .unwrap();
        }
    }
}

#[test]
pub(crate) fn shared_field_reports_reject_stale_invalid_and_loaded_descriptors() {
    for fault in 0..6 {
        let source = if fault == 5 {
            "n:1;p:&n;r:{->p;->tag:true};q:&r;x:q.tag"
        } else {
            "n:1;p:&n;r:{->p;->tag:true};x:r.tag"
        };
        let (mut checker, mut reports) = checked(source, false);
        let id = *checker.fields.keys().next().unwrap();
        let bad = match fault {
            0 => None,
            1 | 2 | 5 => Some(ScalarKind::Bool),
            3 => Some(ScalarKind::Int {
                bits: 7,
                signed: true,
            }),
            4 => Some(ScalarKind::Float { bits: 16 }),
            _ => unreachable!(),
        };
        if fault >= 2 {
            checker.fields.get_mut(&id).unwrap().shared_primary = bad;
        }
        if fault != 2 {
            let (_, Effect::Field { shared_primary, .. }) = reports.effects.get_mut(&id).unwrap()
            else {
                panic!()
            };
            *shared_primary = bad;
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .validate_field_report(&reports, id, 0, &reports.effects[&id].1, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_field_reports_merge_atomically_with_exact_limits_and_no_payload() {
    let (mut checker, mut reports) = checked("n:1;p:&n;r:{->p;->tag:true};x:r.tag", false);
    let id = *checker.fields.keys().next().unwrap();
    let stage = checker
        .field_effect_stage(&reports, 0, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    checker
        .record_field_effect(stage, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for source in [
        None,
        Some(ScalarKind::Bool),
        Some(ScalarKind::Float { bits: 16 }),
    ] {
        let mut changed = stage;
        changed.shared_primary = source;
        assert!(
            checker
                .record_field_effect(changed, &mut effects, 1, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, expected);
    }
    let mut loaded = stage;
    loaded.load = true;
    assert!(
        checker
            .record_field_effect(loaded, &mut effects, 1, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(effects, expected);
    reports.entries.get_mut(&0).unwrap().1.ports =
        vec![Port::Operation(id), Port::Normal(id), Port::Operation(id)];
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let expected = checker
        .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(expected.len(), 1);
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 0, 0, 0)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.operation_effects_limited(&reports, Span::default(), 1, 0, 0);
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
