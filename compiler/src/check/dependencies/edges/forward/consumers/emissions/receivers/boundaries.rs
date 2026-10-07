use super::*;

#[test]
pub(crate) fn receiver_emission_links_reject_late_origins_and_unobserved_layout_faults() {
    for init in ["{->3;->z:9;->a:true}", "3.{->$;->z:9;->a:true}"] {
        for fault in 0..15 {
            let source = format!(
                "early:{{->n:1}}.n;{}",
                SOURCE.replace("3.{->$;->z:9;->a:true}", init)
            );
            let (mut checker, mut reports) = checked(&source);
            let (&id, op) = checker
                .emissions
                .iter()
                .find(|(_, op)| op.composed.is_some())
                .unwrap();
            let (input, last) = (op.input, op.targets[2].id);
            let slot = reports.slot_uses[&Port::Emission(last)].1;
            let local = checker
                .dispatch_ops
                .values()
                .find(|op| matches!(op.receiver, Shape::Record { .. }))
                .unwrap()
                .local;
            let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.initialized = vec![true, false, false];
            op.result = false;
            match fault {
                0 => {
                    let row = &mut reports.results.get_mut(&slot.block).unwrap().1;
                    row.consumer = None;
                    row.dispatch = None;
                }
                1 => {
                    reports
                        .results
                        .get_mut(&slot.block)
                        .unwrap()
                        .1
                        .slots
                        .as_mut()
                        .unwrap()
                        .pop();
                }
                2 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
                3 => {
                    let Layout::Slots(slots) =
                        &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                    else {
                        panic!()
                    };
                    slots[2].field = Some("wrong".into());
                }
                4 => {
                    op.targets[2].field = Some("wrong".into());
                    checker.emissions.get_mut(&id).unwrap().targets[2].field = Some("wrong".into());
                }
                5 => {
                    op.initialized.pop();
                }
                6 => op.input = usize::MAX,
                7 => reports.effects.get_mut(&id).unwrap().0 += 1,
                8 => {
                    checker.emission_sources.insert(last, (id, 0));
                }
                9 => {
                    checker
                        .bodies
                        .get_mut(&slot.block)
                        .unwrap()
                        .completion
                        .result = Shape::Record { fields: 9 }
                }
                10 => {
                    op.composed.as_mut().unwrap().count += 1;
                    checker
                        .emissions
                        .get_mut(&id)
                        .unwrap()
                        .composed
                        .as_mut()
                        .unwrap()
                        .count += 1;
                }
                11 => checker.points[input].parent = None,
                12 => {
                    let read = *checker
                        .local_reads
                        .iter()
                        .rev()
                        .find(|(_, read)| read.local == local)
                        .unwrap()
                        .0;
                    checker.points[read].parent = Some(read);
                }
                13 => reports.receivers.get_mut(&local).unwrap().0 += 1,
                14 => reports.receivers.get_mut(&local).unwrap().1 = usize::MAX,
                _ => unreachable!(),
            }
            let before = format!(
                "{reports:?}{:?}{:?}{:?}",
                checker.bodies,
                checker.emissions,
                checker.edge_counts()
            );
            assert!(
                checker
                    .slot_uses(&reports, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "{init}, {fault}"
            );
            assert_eq!(
                format!(
                    "{reports:?}{:?}{:?}{:?}",
                    checker.bodies,
                    checker.emissions,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}

#[test]
pub(crate) fn receiver_emission_links_bound_maps_work_duplicates_and_missing_evidence() {
    for missing in 0..3 {
        let (mut checker, mut reports) = checked(&format!("early:{{->n:1}}.n;{SOURCE}"));
        let local = checker
            .dispatch_ops
            .values()
            .find(|op| matches!(op.receiver, Shape::Record { .. }))
            .unwrap()
            .local;
        match missing {
            0 => {
                reports.eligible.remove(&local);
            }
            1 => {
                reports.receivers.remove(&local);
            }
            2 => {
                checker.proofs.mutable.insert(local);
            }
            _ => unreachable!(),
        }
        let expected: Uses = reports
            .slot_uses
            .iter()
            .filter(|(port, _)| matches!(port, Port::Operation(_)))
            .map(|(&port, &slot)| (port, slot))
            .collect();
        assert_eq!(expected.len(), 1);
        let before = format!("{reports:?}");
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected
        );
        assert_eq!(format!("{reports:?}"), before);
    }
    let (mut checker, mut reports) = checked(&format!("early:{{->n:1}}.n;{SOURCE}"));
    let expected = reports.slot_uses.clone();
    let effects = reports.effects.clone();
    assert_eq!(expected.len(), 4);
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert_eq!(reports.effects, effects);
    let limit = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + expected.len();
    reports.parts = 0;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        expected
    );
    let work = checker.flow.work - start;
    assert!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit - 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.slot_uses_limited(&reports, Span::default(), limit);
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn seeded_receiver_emission_links_reject_mixed_receiver_initializer_cycles() {
    let (mut checker, mut reports) =
        checked("r:3.{->$;->tag:true};out:r.{alias:(($));copy:{->alias}}");
    let alias = checker.local_reads.values().last().unwrap().local;
    let id = *checker
        .local_reads
        .iter()
        .find(|(_, read)| read.local != alias && !reports.receivers.contains_key(&read.local))
        .unwrap()
        .0;
    let read = checker.local_reads.get_mut(&id).unwrap();
    read.local = alias;
    read.storage = alias;
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    *local = alias;
    *storage = alias;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn receiver_emission_links_keep_source_receiver_and_emission_control_independent() {
    for source in [false, true] {
        for receiver in [false, true] {
            for emission in [false, true] {
                let (mut checker, mut reports) = checked(SOURCE);
                for (&id, op) in &mut checker.dispatch_ops {
                    let control = if matches!(op.receiver, Shape::Record { .. }) {
                        receiver
                    } else {
                        source
                    };
                    op.control = control;
                    let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    observed.control = control;
                }
                for (&id, op) in &mut checker.emissions {
                    op.control = emission;
                    let (_, Effect::Emission(observed)) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    observed.control = emission;
                }
                let before = format!("{reports:?}{:?}", checker.edge_counts());
                assert_eq!(
                    checker.slot_uses(&reports, Span::default()).unwrap(),
                    reports.slot_uses
                );
                assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
            }
        }
    }
}
