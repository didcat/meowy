use super::*;

#[test]
pub(crate) fn shared_local_emissions_bound_exact_maps_work_and_duplicate_visits() {
    let (mut checker, mut reports) = checked(boundaries::SOURCE);
    let effects = reports.effects.clone();
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    assert_eq!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap(),
        effects
    );
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 5);
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
pub(crate) fn shared_local_emissions_keep_bind_read_and_emission_control_independent() {
    let (mut checker, mut reports) = checked(SOURCE);
    for binding in [false, true] {
        for read in [false, true] {
            for emission in [false, true] {
                for (&id, op) in &mut checker.operations {
                    op.control = binding;
                    let (_, Effect::Storage { control, .. }) =
                        reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    *control = binding;
                }
                for (&id, op) in &mut checker.local_reads {
                    op.control = read;
                    let (_, Effect::Read { control, .. }) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    *control = read;
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
                assert_eq!(
                    checker
                        .candidate_walk_report(&reports, Span::default())
                        .unwrap()
                        .0,
                    reports.candidate_walk
                );
                assert_eq!(
                    checker
                        .direct_walk_report(&reports, Span::default())
                        .unwrap()
                        .0,
                    reports.expanded_walk
                );
                assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
            }
        }
    }
}

#[test]
pub(crate) fn shared_local_emissions_preserve_multiple_histories_and_extra_destination_slots() {
    let (checker, reports) = checked(
        "n:1;p:&n;flag:=true;r:{->p;|flag|->a:1;|!flag|->a:2;->z:[true]};copy:r;out:{->copy;->extra:true}",
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

#[test]
pub(crate) fn shared_local_emissions_share_exact_hop_limits_across_copy_wrappers() {
    let mut base = None;
    for groups in [0, 4, 8] {
        let source = format!(
            "n:1;p:&n;r:{{->p;->a:true}};copy:{}r{};out:{{->copy}}",
            "(".repeat(groups),
            ")".repeat(groups)
        );
        let (mut checker, reports) = checked(&source);
        let op = checker
            .emissions
            .values()
            .find(|op| op.composed.is_some())
            .unwrap();
        let composed = op.composed.unwrap();
        let input = op.input;
        let block = reports.slot_uses[&Port::Emission(op.targets[0].id)].1.block;
        let expected = reports.results[&block].1.consumer;
        let target = grouped::Target::SharedLocal(SharedRecord {
            primary: composed.shared_primary.unwrap(),
            fields: composed.count,
        });
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut minimum = None;
        for limit in 0..64 {
            match checker.grouped_source_limited(&reports, input, 0, Span::default(), limit, target)
            {
                Ok(actual) => {
                    assert_eq!(actual, expected);
                    minimum = Some(limit);
                    break;
                }
                Err(error) => assert!(error.message.contains("budget")),
            }
        }
        let hops = minimum.unwrap();
        assert_eq!(hops, *base.get_or_insert(hops) + groups);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
