use super::{super::tests::checked, *};

#[test]
pub(crate) fn element_effects_share_exact_work_effect_and_source_payload_limits() {
    let source = "r:{->inner:{->xs:[1]}};p:&(r.inner.xs[1])";
    for (missing, parts, pass) in [(0, 12, true), (0, 11, false), (1, 12, false)] {
        let (mut checker, mut reports) = checked(source, false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let expected = reports.effects.clone();
        let ops = checker.elements.clone();
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
        assert_eq!(checker.elements, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn element_effects_preserve_partial_records_and_payloads_on_conflicts() {
    let (mut checker, _) = checked("r:{->inner:{->xs:[1]}};p:&(r.inner.xs[1])", false);
    let id = *checker.elements.first_key_value().unwrap().0;
    let address = Port::Address { point: id, step: 0 };
    let mut effects = Effects::new();
    let mut parts = 3;
    assert!(
        checker
            .record_element_effect(0, address, &mut effects, 1, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 3);
    parts = 4;
    assert!(
        checker
            .record_element_effect(0, address, &mut effects, 0, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 4);
    checker
        .record_element_effect(0, address, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(parts, 0);
    let expected = effects.clone();
    for fault in 0..14 {
        let mut effects = expected.clone();
        let (owner, Effect::Element(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        let ElementSource::Place {
            place,
            storage,
            counts,
        } = &mut op.source
        else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.parent += 1,
            2 => op.source = ElementSource::View,
            3 => place.root += 1,
            4 => *storage += 1,
            5 => place.fields[0] += 1,
            6 => counts[0] += 1,
            7 => op.access.position += 1,
            8 => op.access.capacity += 1,
            9 => op.access.length = Some(1),
            10 => op.access.site += 1,
            11 => op.access.may_return = false,
            12 => op.control = true,
            13 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_element_effect(
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
    for port in [Port::Entry(id), Port::Address { point: id, step: 1 }] {
        assert!(
            checker
                .record_element_effect(0, port, &mut effects, 1, &mut parts, Span::default())
                .is_err()
        );
        assert_eq!(effects, expected);
    }
    checker
        .record_element_effect(0, address, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    assert_eq!(parts, 0);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_element_effect(
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
pub(crate) fn element_effects_bound_maximum_paths_and_zero_payload_sources() {
    let (mut checker, mut reports) = checked("r:{->xs:[1]};p:&(r.xs[1])", false);
    let id = *checker.elements.first_key_value().unwrap().0;
    let ElementSource::Place { place, counts, .. } =
        &mut checker.elements.get_mut(&id).unwrap().source
    else {
        panic!()
    };
    let len = crate::list::MAX_WRITE_PATH;
    place.fields = vec![0; len];
    *counts = vec![1; len];
    let address = Port::Address { point: id, step: 0 };
    reports.entries.get_mut(&0).unwrap().1.ports = vec![address, address];
    let before = reports.effects.clone();
    let effects = checker
        .operation_effects_limited(&reports, Span::default(), 1, len * 2, 0)
        .unwrap();
    let (_, Effect::Element(op)) = &effects[&id] else {
        panic!()
    };
    assert_eq!(op.source, checker.elements[&id].source);
    assert!(op.address && !op.acquired && !op.result);
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 1, len * 2 - 1, 0)
            .is_err()
    );
    let ElementSource::Place { place, counts, .. } =
        &mut checker.elements.get_mut(&id).unwrap().source
    else {
        panic!()
    };
    place.fields.push(0);
    counts.push(1);
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 1, usize::MAX, 0)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(reports.effects, before);
    for source in ["xs:[1];v:&xs;p:&(v[1])", "x:*(&([1][1]))"] {
        let (mut checker, mut reports) = checked(source, false);
        let id = *checker.elements.first_key_value().unwrap().0;
        reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Normal(id)];
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
            .unwrap();
        let (_, Effect::Element(op)) = &effects[&id] else {
            panic!()
        };
        assert!(op.result && !op.address && !op.acquired);
    }
}
