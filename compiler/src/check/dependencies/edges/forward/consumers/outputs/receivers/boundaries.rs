use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "d:@\"debug\";seed:{->1;->tag:false};good:-seed;r:{->3;->tag:true};out:r.{d.print(\"a{$}b{$}\")}";

#[test]
pub(crate) fn receiver_output_sources_reject_late_shape_layout_origin_and_registration_faults() {
    for init in ["{->3;->tag:true}", "3.{->$;->tag:true}"] {
        for fault in 0..15 {
            let source = format!(
                "d:@\"debug\";seed:{{->1;->tag:false}};good:-seed;r:{init};out:r.{{d.print(\"a{{$}}b{{$}}\")}}"
            );
            let (mut checker, mut reports) = checked(&source);
            let id = *checker.outputs.keys().next().unwrap();
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
                    checker.outputs.get_mut(&id).unwrap().parts[3]
                        .as_mut()
                        .unwrap()
                        .source = source;
                    let (_, Effect::Output(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.parts.get_mut(&3).unwrap().input.as_mut().unwrap().source = source;
                }
                6 => {
                    let row = &mut reports.results.get_mut(&slot.block).unwrap().1;
                    row.consumer = None;
                    row.dispatch = None;
                }
                7 => checker.points[checker.outputs[&id].parts[3].unwrap().point].owner += 1,
                8 => {
                    reports.index.operations.remove(&id);
                }
                9 => {
                    checker
                        .outputs
                        .get_mut(&id)
                        .unwrap()
                        .edges
                        .last_mut()
                        .unwrap()
                        .route = Route::Next
                }
                10..=12 => {
                    let Layout::Slots(slots) =
                        &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                    else {
                        panic!()
                    };
                    match fault {
                        10 => slots[0].mutable = true,
                        11 => slots[0].field = Some("wrong".into()),
                        12 => slots[0].shape = Shape::Scalar(ScalarKind::Bool),
                        _ => unreachable!(),
                    }
                }
                13 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
                14 => {
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
                checker.outputs,
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
                    checker.outputs,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}

#[test]
pub(crate) fn receiver_output_sources_bound_shared_maps_work_and_duplicate_observations() {
    for missing in [false, true] {
        let (mut checker, mut reports) = checked(SOURCE);
        let local = checker.dispatch_ops.values().next().unwrap().local;
        if missing {
            reports.eligible.remove(&local);
        } else {
            reports.receivers.remove(&local);
        }
        let expected: Uses = reports.slot_uses.iter().filter_map(|(&port, &value)| {
            matches!(port, Port::Projection { point, .. } if matches!(reports.effects[&point].1, Effect::Coercion(_))).then_some((port, value))
        }).collect();
        assert_eq!(expected.len(), 1);
        let before = format!("{reports:?}");
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected
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
pub(crate) fn seeded_receiver_output_sources_reject_mixed_receiver_initializer_cycles() {
    let (mut checker, mut reports) =
        checked("d:@\"debug\";r:{->3;->tag:true};out:r.{alias:(($));d.print(alias)}");
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
pub(crate) fn receiver_output_sources_keep_source_receiver_and_output_control_independent() {
    for source in [false, true] {
        for receiver in [false, true] {
            for output in [false, true] {
                let (mut checker, mut reports) =
                    checked("d:@\"debug\";r:3.{->$;->tag:true};out:r.{d.print($)}");
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
                for (&id, op) in &mut checker.outputs {
                    op.control = output;
                    let (_, Effect::Output(observed)) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    observed.control = output;
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
pub(crate) fn receiver_output_sources_revalidate_checked_parts_after_a_stop() {
    let (mut checker, reports) = checked(
        "d:@\"debug\";r:{->3;->tag:true};out:r.{d.print(\"a{$}b{d.panic(\"stop\")}tail{$}\")}",
    );
    let (&id, op) = checker
        .outputs
        .iter()
        .find(|(_, op)| op.parts.len() > 1)
        .unwrap();
    let (_, Effect::Output(output)) = &reports.effects[&id] else {
        panic!()
    };
    assert!(!output.parts.contains_key(&5));
    checker.points[op.parts[5].unwrap().point].owner += 1;
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
