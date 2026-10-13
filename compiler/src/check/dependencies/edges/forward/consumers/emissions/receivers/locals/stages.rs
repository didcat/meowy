use super::*;

#[test]
pub(crate) fn shared_local_receiver_emissions_keep_source_receiver_target_and_result_stages_independent()
 {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
    let targets = op.targets.clone();
    let expected = reports.slot_uses.clone();
    let slot = expected[&Port::Emission(targets[0].id)].1;
    let source = reports.results[&slot.block].1.dispatch.unwrap();
    let receiver = reports
        .receivers
        .values()
        .find(|row| row.2.is_some())
        .unwrap()
        .1;
    reports.blocks.remove(&targets[0].block);
    reports.results.remove(&targets[0].block);
    for source_init in [false, true] {
        for published in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                panic!()
            };
            op.initialized = source_init;
            op.result = published;
            for initialized in [false, true] {
                for completed in [false, true] {
                    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap()
                    else {
                        panic!()
                    };
                    op.initialized = initialized;
                    op.result = completed;
                    for result in [false, true] {
                        for selected in [
                            [false; 3],
                            [true, false, false],
                            [false, false, true],
                            [true; 3],
                        ] {
                            let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap()
                            else {
                                panic!()
                            };
                            op.initialized = selected.to_vec();
                            op.result = result;
                            let wanted = targets
                                .iter()
                                .enumerate()
                                .filter_map(|(index, target)| {
                                    let port = Port::Emission(target.id);
                                    (published && initialized && selected[index])
                                        .then_some((port, expected[&port]))
                                })
                                .collect::<Uses>();
                            assert_eq!(
                                checker.slot_uses(&reports, Span::default()).unwrap(),
                                wanted
                            );
                        }
                    }
                }
            }
        }
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn shared_local_receiver_emissions_preserve_kinds_and_reached_prefixes() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        for tail in [
            "copy:{->$;d.panic(\"stop\")}",
            "copy:{->$};d.panic(\"stop\");later:{->$}",
        ] {
            let source =
                format!("d:@\"debug\";n<{ty}>:{value};p:&n;r:{{->p;->tag:true}};out:r.{{{tail}}}");
            let (checker, reports) = checked(&source);
            assert_eq!(reports.slot_uses.len(), 2);
            let op = checker
                .emissions
                .values()
                .find(|op| op.composed.is_some())
                .unwrap();
            let primary = op.composed.unwrap().shared_primary.unwrap();
            let slot = reports.slot_uses[&Port::Emission(op.targets[0].id)].1;
            let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(layout[0].shape, Shape::SharedScalar(primary));
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
        }
    }
}

#[test]
pub(crate) fn shared_local_receiver_emissions_preserve_multiple_histories_and_unknown_aggregates() {
    let (checker, reports) = checked(
        "n:1;p:&n;flag:=true;r:{->p;|flag|->a:1;|!flag|->a:2;->z:[true]};out:r.{copy:{->$;->extra:true}}",
    );
    let op = checker
        .emissions
        .values()
        .find(|op| op.composed.is_some())
        .unwrap();
    assert_eq!(op.targets.len(), 3);
    let slots: Vec<_> = op
        .targets
        .iter()
        .map(|target| reports.slot_uses[&Port::Emission(target.id)].1)
        .collect();
    let sources = reports.results[&slots[0].block].1.slots.as_ref().unwrap();
    assert_eq!(sources[0], Sources::Unknown);
    assert!(matches!(&sources[1], Sources::Candidates(values) if values.len() == 2));
    assert_eq!(sources[2], Sources::Unknown);
    let Layout::Slots(layout) = &checker.bodies[&op.targets[0].block].layout else {
        panic!()
    };
    assert_eq!(layout.len(), 4);
    assert!(slots.iter().all(|slot| slot.block != op.targets[0].block));
}
