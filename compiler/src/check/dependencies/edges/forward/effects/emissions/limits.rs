use super::{super::tests::checked, *};

#[test]
pub(crate) fn emission_effects_share_exact_work_map_and_payload_limits() {
    let source = "f<int32>:(x<int32>){->x};xs:[f(1),2];r:{->name:=3};r.name=4";
    for (missing, parts, pass) in [(0, 18, true), (0, 17, false), (1, 18, false)] {
        let (mut checker, mut reports) = checked(source, false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let expected = reports.effects.clone();
        let sources = checker.emission_sources.clone();
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
        assert_eq!(checker.emission_sources, sources);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn emission_effects_keep_partial_records_atomic_on_conflicts_and_exhaustion() {
    let (mut checker, _) = checked("source:{->7;->a:1;->b:true};copy:{->source}", false);
    let id = *checker
        .emissions
        .iter()
        .find(|(_, op)| op.targets.len() == 3)
        .unwrap()
        .0;
    let stage = (id, Some(0));
    let mut effects = Effects::new();
    let mut parts = 7;
    assert!(
        checker
            .record_emission_effect(0, stage, &mut effects, 1, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 7);
    parts = 8;
    assert!(
        checker
            .record_emission_effect(0, stage, &mut effects, 0, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 8);
    checker
        .record_emission_effect(0, stage, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(parts, 0);
    let expected = effects.clone();
    for fault in 0..15 {
        let mut effects = expected.clone();
        let (owner, Effect::Emission(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.input += 1,
            2 => op.composed = None,
            3 => op.composed.as_mut().unwrap().local += 1,
            4 => op.composed.as_mut().unwrap().count += 1,
            5 => op.targets.clear(),
            6 => op.targets[1].id += 1,
            7 => op.targets[1].block += 1,
            8 => op.targets[1].field = Some("changed".into()),
            9 => op.targets[1].projection = Projection::Field(1),
            10 => op.targets[1].alias = Some(0),
            11 => op.targets[1].storage = Some(0),
            12 => op.control = true,
            13 => op.initialized.clear(),
            14 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_emission_effect(0, (id, None), &mut effects, 1, &mut parts, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
        assert_eq!(parts, 0);
    }
    checker
        .record_emission_effect(0, stage, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    assert!(
        checker
            .record_emission_effect(
                0,
                (id, Some(3)),
                &mut effects,
                1,
                &mut parts,
                Span::default()
            )
            .is_err()
    );
    assert_eq!(effects, expected);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_emission_effect(0, (id, None), &mut effects, 1, &mut parts, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(effects, expected);
    assert_eq!(parts, 0);
}

#[test]
pub(crate) fn emission_effects_bound_field_name_bytes_and_target_descriptors_before_copying() {
    let (mut checker, mut reports) = checked("row:{->x:1}", false);
    let (&id, op) = checker.emissions.first_key_value().unwrap();
    let target = op.targets[0].id;
    let alias = op.targets[0].alias.unwrap();
    let name = "x".repeat(MAX_EDGES - 2);
    checker.emissions.get_mut(&id).unwrap().targets[0].field = Some(name.clone());
    checker.proofs.aliases.get_mut(&alias).unwrap().field = name;
    reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Emission(target)];
    let before = reports.effects.clone();
    for missing in [0, 1] {
        checker.flow.work = 0;
        let result =
            checker.operation_effects_limited(&reports, Span::default(), 1, MAX_EDGES - missing, 0);
        assert_eq!(result.is_ok(), missing == 0);
        if let Ok(effects) = result {
            let (_, Effect::Emission(op)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(op.targets[0].field.as_ref().unwrap().len(), MAX_EDGES - 2);
            assert_eq!(op.initialized, [true]);
            assert!(!op.result);
        }
        assert_eq!(reports.effects, before);
    }
    checker.emissions.get_mut(&id).unwrap().targets[0].field = Some("x".repeat(MAX_EDGES + 1));
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 1, usize::MAX, 0)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(reports.effects, before);
    let target = Target {
        id: 0,
        block: 0,
        field: None,
        projection: Projection::Value,
        alias: None,
        storage: None,
    };
    let mut targets = vec![target.clone(); MAX_TARGETS];
    let mut flow = crate::flow::Flow::new();
    assert_eq!(name_bytes(&targets, &mut flow, Span::default()).unwrap(), 0);
    targets.push(target);
    assert!(
        name_bytes(&targets, &mut flow, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
}
