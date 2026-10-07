use super::*;
use crate::check::dependencies::{ScalarKind, SequenceSource};

#[test]
pub(crate) fn receiver_list_sources_reject_late_shape_layout_origin_and_registration_faults() {
    for init in ["{->3;->tag:true}", "3.{->$;->tag:true}"] {
        for fault in 0..16 {
            let source = SOURCE.replace("3.{->$;->tag:true}", init);
            let (mut checker, mut reports) = checked(&source);
            let id = *checker.lists.keys().next().unwrap();
            let slot = reports.slot_uses[&Port::Projection { point: id, step: 3 }].1;
            assert_eq!(reports.slot_uses.len(), 3);
            match fault {
                0..=5 => {
                    let source = match fault {
                        0 => None,
                        1 => Some(Shape::Scalar(ScalarKind::Bool)),
                        2 => Some(Shape::Scalar(ScalarKind::Int {
                            bits: 8,
                            signed: true,
                        })),
                        3 => Some(Shape::Scalar(ScalarKind::Int {
                            bits: 32,
                            signed: false,
                        })),
                        4 => Some(Shape::Never),
                        5 => Some(Shape::Scalar(ScalarKind::Int {
                            bits: 7,
                            signed: true,
                        })),
                        _ => unreachable!(),
                    };
                    checker.list_inputs.get_mut(&id).unwrap()[3].source = source;
                    let (_, Effect::List(list)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    list.inputs[3].source = source;
                }
                6 => {
                    let row = &mut reports.results.get_mut(&slot.block).unwrap().1;
                    row.consumer = None;
                    row.dispatch = None;
                }
                7 => {
                    reports.index.operations.remove(&id);
                }
                8 => checker.points[checker.list_inputs[&id][3].point].owner += 1,
                9..=11 => {
                    let Layout::Slots(slots) =
                        &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                    else {
                        panic!()
                    };
                    match fault {
                        9 => slots[0].mutable = true,
                        10 => slots[0].field = Some("wrong".into()),
                        11 => slots[0].shape = Shape::Scalar(ScalarKind::Bool),
                        _ => unreachable!(),
                    }
                }
                12 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
                13 => {
                    let (_, Effect::List(list)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    list.inputs[3].point = usize::MAX;
                }
                14 => checker
                    .endpoints
                    .get_mut(&SequenceSource::Expr(id))
                    .unwrap()
                    .clear(),
                15 => {
                    let local = checker
                        .dispatch_ops
                        .values()
                        .find(|op| matches!(op.receiver, Shape::Record { .. }))
                        .unwrap()
                        .local;
                    let read = *checker
                        .local_reads
                        .iter()
                        .rev()
                        .find(|(_, read)| read.local == local)
                        .unwrap()
                        .0;
                    checker.points[read].parent = Some(read);
                }
                _ => unreachable!(),
            }
            let before = format!(
                "{reports:?}{:?}{:?}",
                checker.list_inputs,
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
                    "{reports:?}{:?}{:?}",
                    checker.list_inputs,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}

#[test]
pub(crate) fn receiver_list_sources_bound_shared_maps_work_and_duplicate_observations() {
    for missing in [false, true] {
        let (mut checker, mut reports) = checked(SOURCE);
        let local = checker
            .dispatch_ops
            .values()
            .find(|op| matches!(op.receiver, Shape::Record { .. }))
            .unwrap()
            .local;
        if missing {
            reports.eligible.remove(&local);
        } else {
            reports.receivers.remove(&local);
        }
        let before = format!("{reports:?}");
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap()
                .is_empty()
        );
        assert_eq!(format!("{reports:?}"), before);
    }
    let (mut checker, mut reports) = checked(SOURCE);
    let expected = reports.slot_uses.clone();
    let effects = reports.effects.clone();
    assert_eq!(expected.len(), 3);
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
pub(crate) fn seeded_receiver_list_sources_reject_mixed_receiver_initializer_cycles() {
    let source = "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:{->3;->tag:true};out:r.{copy:(($));|v<boolean>|xs<T[2]><U[2]>:[copy,v]}}";
    let (mut checker, mut reports) = checked(source);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let (point, owner) = (checker.list_inputs[&id][0].point, op.owner);
    let read = checker
        .unchanged_narrowing_input(&reports, point, owner, Span::default())
        .unwrap()
        .unwrap();
    let copy = checker.local_reads[&read].local;
    let id = *checker
        .local_reads
        .iter()
        .find(|(_, read)| {
            read.local != copy
                && !reports.receivers.contains_key(&read.local)
                && reports.eligible.contains(&read.local)
        })
        .unwrap()
        .0;
    let read = checker.local_reads.get_mut(&id).unwrap();
    read.local = copy;
    read.storage = copy;
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    *local = copy;
    *storage = copy;
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
pub(crate) fn receiver_list_sources_keep_source_receiver_and_list_control_independent() {
    for source in [false, true] {
        for receiver in [false, true] {
            for list in [false, true] {
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
                for (&id, op) in &mut checker.lists {
                    op.control = list;
                    let (_, Effect::List(observed)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    observed.control = list;
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

#[test]
pub(crate) fn receiver_list_sources_revalidate_unobserved_elements_after_a_stop() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:{->3;->tag:true};out:r.{|v<boolean>|xs<T[4]><U[4]>:[$,v,stop(),$]}}";
    let (mut checker, reports) = checked(source);
    let id = *checker.lists.keys().next().unwrap();
    let (_, Effect::List(list)) = &reports.effects[&id] else {
        panic!()
    };
    assert!(!list.inputs[3].projected && !list.inputs[3].converted);
    assert_eq!(reports.slot_uses.len(), 1);
    checker.points[checker.list_inputs[&id][3].point].owner += 1;
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
