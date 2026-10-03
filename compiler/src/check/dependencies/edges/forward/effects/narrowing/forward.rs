use super::{super::tests::checked, *};

#[test]
pub(crate) fn unchanged_narrowing_inputs_retain_local_field_owner_and_control_roots() {
    let source = "flag:false;r:{->n:1};x:r.n;|flag|r;f<int32>:(v<int32>){->v}";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, true);
    let ids: Vec<_> = checker
        .narrowings
        .iter()
        .map(|(&id, op)| (id, op.owner, op.input))
        .collect();
    assert!(ids.len() >= 4);
    assert!(checker.narrowings.values().any(|op| op.control));
    assert!(ids.iter().any(|(_, owner, _)| *owner != 0));
    assert!(
        ids.iter()
            .any(|(_, _, input)| matches!(reports.effects[input].1, Effect::Field { .. }))
    );
    for (id, owner, input) in ids {
        assert_eq!(
            checker
                .unchanged_narrowing_input(&reports, id, owner, Span::default())
                .unwrap(),
            Some(input)
        );
    }
}

#[test]
pub(crate) fn unchanged_narrowing_inputs_require_result_and_exclude_changed_or_stopped_stages() {
    let source = "v:1;x:v;f:(v<int32><null>){|v<int32>|v};g<never>:(v<never>){->v}";
    crate::compile(source).unwrap();
    let (mut checker, mut reports) = checked(source, false);
    let ids: Vec<_> = checker.narrowings.keys().copied().collect();
    assert!(checker.narrowings.values().any(|op| op.changed));
    assert!(checker.narrowings.values().any(|op| !op.normal));
    for id in ids {
        let op = &checker.narrowings[&id];
        let (owner, input) = (op.owner, op.input);
        let expected =
            (!op.changed && op.normal && reports.effects.contains_key(&id)).then_some(input);
        assert_eq!(
            checker
                .unchanged_narrowing_input(&reports, id, owner, Span::default())
                .unwrap(),
            expected
        );
        if expected.is_some() {
            let (_, Effect::Narrowing(observed)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            observed.result = false;
            assert_eq!(
                checker
                    .unchanged_narrowing_input(&reports, id, owner, Span::default())
                    .unwrap(),
                None
            );
            reports.effects.remove(&id);
            assert_eq!(
                checker
                    .unchanged_narrowing_input(&reports, id, owner, Span::default())
                    .unwrap(),
                None
            );
        }
    }
    let other = *checker.operations.first_key_value().unwrap().0;
    assert_eq!(
        checker
            .unchanged_narrowing_input(&reports, other, 0, Span::default())
            .unwrap(),
        None
    );
}
