use super::{super::tests::checked, *};

#[test]
pub(crate) fn emission_effects_reject_corrupt_composition_slots_before_any_observation() {
    for stage in 0..4 {
        for fault in 0..18 {
            let (mut checker, mut reports) = checked("v:{->7;->a:1;->b:true};copy:{->v}", false);
            let (&id, op) = checker
                .emissions
                .iter()
                .find(|(_, op)| op.targets.len() == 3)
                .unwrap();
            let first = op.targets[0].id;
            let field = op.targets[1].id;
            let composed = op.composed.unwrap();
            let port = if stage == 3 {
                Port::Normal(id)
            } else {
                Port::Emission(op.targets[stage].id)
            };
            let alias = checker.proofs.aliases.values().next().unwrap().clone();
            let op = checker.emissions.get_mut(&id).unwrap();
            match fault {
                0 => op.composed = None,
                1 => op.composed.as_mut().unwrap().count = 1,
                2 => op.composed.as_mut().unwrap().local = reports.locals,
                3 => op.targets[0].projection = Projection::Value,
                4 => op.targets[0].field = Some("a".into()),
                5 => op.targets[1].projection = Projection::Field(1),
                6 => op.targets[2].field = None,
                7 => op.targets[2].field = Some("a".into()),
                8 => op.targets[2].block += 1,
                9 => {
                    op.targets[1].alias = Some(0);
                    op.targets[1].storage = Some(0);
                }
                10 => op.targets[1].id = first,
                11 => op.targets.swap(1, 2),
                12 => op.edges[1].route = Route::Checked,
                13 => {
                    checker.emission_sources.insert(field, (id, 2));
                }
                14 => {
                    checker.proofs.aliases.insert(composed.local, alias);
                }
                15 => op.targets.push(op.targets[2].clone()),
                16 => op.targets[1].storage = Some(0),
                17 => op.targets[1].field = Some(String::new()),
                _ => unreachable!(),
            }
            reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
            let before = reports.effects.clone();
            let sources = checker.emission_sources.clone();
            assert!(
                checker
                    .operation_effects(&reports, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "stage {stage} fault {fault}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.emission_sources, sources);
        }
    }
}

#[test]
pub(crate) fn emission_effects_reject_sibling_foreign_and_cyclic_target_scopes() {
    for fault in 0..5 {
        let (mut checker, reports) =
            checked("row:'out{{'out->1}};sibling:{->2};f<int32>:(){->3}", false);
        let (&id, _) = checker.emissions.first_key_value().unwrap();
        let block = checker.points[id].block.unwrap();
        let parent = checker.bodies[&block].parent.unwrap();
        let sibling = checker.emissions.values().nth(1).unwrap().targets[0].block;
        let foreign = checker
            .emissions
            .values()
            .find(|op| op.owner != 0)
            .unwrap()
            .targets[0]
            .block;
        match fault {
            0 => checker.emissions.get_mut(&id).unwrap().targets[0].block = sibling,
            1 => checker.emissions.get_mut(&id).unwrap().targets[0].block = foreign,
            2 => checker.bodies.get_mut(&block).unwrap().parent = None,
            3 => checker.points[parent].block = Some(block),
            4 => checker.points[parent].complete = false,
            _ => unreachable!(),
        }
        let before = reports.effects.clone();
        assert!(
            checker
                .operation_effects(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(reports.effects, before);
    }
}

#[test]
pub(crate) fn emission_effects_keep_canonical_alias_storage_and_ordinary_errors() {
    let source =
        "flag:=false;row:'out{|flag|{'out->value:=1;value=2};|!flag|{'out->value:=3;value=4}}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let slots: Vec<_> = checker
        .emissions
        .iter()
        .map(|(id, op)| {
            let (_, Effect::Emission(observed)) = &reports.effects[id] else {
                panic!()
            };
            assert_eq!(observed.targets, op.targets);
            &observed.targets[0]
        })
        .collect();
    assert_eq!(slots.len(), 2);
    assert_ne!(slots[0].alias, slots[1].alias);
    assert_eq!(slots[0].storage, slots[1].storage);
    for (source, code) in [
        ("->x:1;->x:2", "E205"),
        ("->x<boolean>:1", "E207"),
        ("'out{f:(){'out->1}}", "E201"),
        ("source:{->x:1};value:{->source;->x:2}", "E205"),
        ("value:{local:1;->&local}", "E303"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
