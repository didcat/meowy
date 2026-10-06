use super::*;

#[test]
pub(crate) fn dispatch_emission_links_reject_late_origins_and_unobserved_layout_faults() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked(&format!("early:{{->n:1}}.n;{SOURCE}"));
        let (&id, op) = checker
            .emissions
            .iter()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let (input, last) = (op.input, op.targets[2].id);
        let slot = reports.slot_uses[&Port::Emission(last)].1;
        let dispatch = reports.results[&slot.block].1.dispatch.unwrap();
        let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.initialized = vec![true, false, false];
        op.result = false;
        match fault {
            0 => reports.results.get_mut(&slot.block).unwrap().1.consumer = Some(dispatch),
            1 => reports.results.get_mut(&slot.block).unwrap().1.dispatch = None,
            2 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
            3 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
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
            "{fault}"
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

#[test]
pub(crate) fn dispatch_emission_links_share_exact_map_and_work_limits_without_payload() {
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
    let before = format!(
        "{reports:?}{:?}{:?}",
        checker.emissions,
        checker.edge_counts()
    );
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
    assert_eq!(
        format!(
            "{reports:?}{:?}{:?}",
            checker.emissions,
            checker.edge_counts()
        ),
        before
    );
}

#[test]
pub(crate) fn seeded_dispatch_emission_links_reject_mixed_initializer_cycles() {
    let (mut checker, mut reports) = checked("r:3.{->$;->tag:true};alias:((r));copy:{->alias}");
    let alias = checker.local_reads.values().last().unwrap().local;
    let id = *checker
        .local_reads
        .iter()
        .find(|(_, read)| !reports.receivers.contains_key(&read.local))
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
pub(crate) fn dispatch_emission_links_preserve_producer_and_consumer_control_independently() {
    for producer in [false, true] {
        for consumer in [false, true] {
            let (mut checker, mut reports) = checked(SOURCE);
            for (&id, op) in &mut checker.dispatch_ops {
                op.control = producer;
                let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.control = producer;
            }
            for (&id, op) in &mut checker.emissions {
                op.control = consumer;
                let (_, Effect::Emission(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.control = consumer;
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
