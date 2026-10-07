use super::*;

pub(super) const SOURCE: &str = "r:{->3;->tag:true};out:r.{good:$+1;bad:$+$}";

#[test]
pub(crate) fn receiver_binary_sources_reject_late_operand_layout_and_receiver_faults() {
    for init in ["{->3;->tag:true}", "3.{->$;->tag:true}"] {
        for fault in 0..15 {
            let source = format!("r:{init};out:r.{{good:$+1;bad:$+$}}");
            let (mut checker, mut reports) = checked(&source);
            let id = *checker.binaries.keys().last().unwrap();
            let slot = reports.slot_uses[&Port::Projection { point: id, step: 1 }].1;
            assert_eq!(reports.slot_uses.len(), 3);
            match fault {
                0 => reports.effects.get_mut(&id).unwrap().0 += 1,
                1 => {
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.inputs.swap(0, 1);
                }
                2 => {
                    let op = checker.binaries.get_mut(&id).unwrap();
                    op.plan.primary = [false; 2];
                    let (_, Effect::Binary(observed)) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    observed.plan = op.plan;
                }
                3 => {
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.projected = [false; 2];
                    op.operation = false;
                    op.result = false;
                }
                4..=7 => {
                    let ty = BinaryClass::Scalar(ScalarKind::Int {
                        bits: match fault {
                            5 => 8,
                            6 => 7,
                            _ => 32,
                        },
                        signed: fault != 4,
                    });
                    let captured = checker.binaries.get_mut(&id).unwrap();
                    captured.types.inputs = [ty; 2];
                    captured.types.result = if fault == 7 {
                        BinaryClass::Scalar(ScalarKind::Bool)
                    } else {
                        ty
                    };
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.types = captured.types;
                }
                8..=10 => {
                    let Layout::Slots(slots) =
                        &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                    else {
                        panic!()
                    };
                    match fault {
                        8 => slots[0].shape = Shape::Scalar(ScalarKind::Bool),
                        9 => slots[0].mutable = true,
                        10 => slots[0].field = Some("wrong".into()),
                        _ => unreachable!(),
                    }
                }
                11 => {
                    let row = &mut reports.results.get_mut(&slot.block).unwrap().1;
                    row.consumer = None;
                    row.dispatch = None;
                }
                12 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
                13 => {
                    reports.index.operations.remove(&id);
                }
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
                checker.binaries,
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
                    checker.binaries,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}

#[test]
pub(crate) fn receiver_binary_sources_preserve_missing_evidence_and_exact_map_work_limits() {
    for missing in [false, true] {
        let (mut checker, mut reports) = checked(SOURCE);
        let local = checker.dispatch_ops.values().next().unwrap().local;
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
    assert_eq!(expected.len(), 3);
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
pub(crate) fn seeded_receiver_binary_sources_reject_mixed_receiver_initializer_cycles() {
    let (mut checker, mut reports) =
        checked("r:{->3;->tag:true};out:r.{alias:(($));good:$+1;bad:alias+1}");
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
pub(crate) fn receiver_binary_sources_keep_source_receiver_and_binary_control_independent() {
    for source in [false, true] {
        for receiver in [false, true] {
            for binary in [false, true] {
                let (mut checker, mut reports) = checked("r:3.{->$;->tag:true};out:r.{x:$+1}");
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
                for (&id, op) in &mut checker.binaries {
                    op.control = binary;
                    let (_, Effect::Binary(observed)) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    observed.control = binary;
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
