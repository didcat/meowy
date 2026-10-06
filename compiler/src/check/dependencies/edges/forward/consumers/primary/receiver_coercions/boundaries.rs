use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

pub(super) const SOURCE: &str = "r:{->3;->tag:true};out:r.{good<int32>:$;bad<int32><null>:$}";

#[test]
pub(crate) fn receiver_coercion_sources_reject_late_types_layouts_and_origins() {
    for init in ["{->3;->tag:true}", "3.{->$;->tag:true}"] {
        for fault in 0..12 {
            let source = format!("r:{init};out:r.{{good<int32>:$;bad<int32><null>:$}}");
            let (mut checker, mut reports) = checked(&source);
            let id = *checker
                .coercions
                .iter()
                .rev()
                .find(|(_, op)| op.primary)
                .unwrap()
                .0;
            let slot = reports.slot_uses[&Port::Projection { point: id, step: 0 }].1;
            match fault {
                0..=4 => {
                    let source = match fault {
                        0 => None,
                        1 => Some(Shape::Scalar(ScalarKind::Bool)),
                        2 => Some(Shape::Scalar(ScalarKind::Int {
                            bits: 32,
                            signed: false,
                        })),
                        3 => Some(Shape::Scalar(ScalarKind::Int {
                            bits: 8,
                            signed: true,
                        })),
                        4 => Some(Shape::Scalar(ScalarKind::Int {
                            bits: 7,
                            signed: true,
                        })),
                        _ => unreachable!(),
                    };
                    checker.coercions.get_mut(&id).unwrap().source = source;
                    let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.source = source;
                }
                5 | 6 => {
                    let Layout::Slots(slots) =
                        &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                    else {
                        panic!()
                    };
                    if fault == 5 {
                        slots[0].mutable = true;
                    } else {
                        slots[0].field = Some("wrong".into());
                    }
                }
                7 => {
                    let row = &mut reports.results.get_mut(&slot.block).unwrap().1;
                    row.consumer = None;
                    row.dispatch = None;
                }
                8 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
                9 => {
                    reports.index.operations.remove(&id);
                }
                10 => {
                    let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.input = usize::MAX;
                }
                11 => {
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
                checker.coercions,
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
                    checker.coercions,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}

#[test]
pub(crate) fn receiver_coercion_sources_preserve_missing_evidence_and_exact_map_work_limits() {
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
    assert_eq!(expected.len(), 2);
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
pub(crate) fn seeded_receiver_coercion_sources_reject_cycles_across_receiver_inputs() {
    let (mut checker, mut reports) =
        checked("r:{->3;->tag:true};out:r.{alias:(($));copy<int32>:alias}");
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
pub(crate) fn receiver_coercion_sources_preserve_source_receiver_and_coercion_control_independently()
 {
    for source in [false, true] {
        for receiver in [false, true] {
            for coercion in [false, true] {
                let (mut checker, mut reports) =
                    checked("r:3.{->$;->tag:true};out:r.{copy<int32>:$}");
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
                for (&id, op) in &mut checker.coercions {
                    op.control = coercion;
                    let (_, Effect::Coercion(observed)) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    observed.control = coercion;
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
pub(crate) fn receiver_coercion_sources_preserve_empty_and_multiple_primary_histories() {
    for (source, count) in [
        ("r:{->tag:true};out:r.{copy<null>:$}", 0),
        (
            "flag:=false;r:{|flag|->3;|!flag|->4;->tag:true};out:r.{copy<int32>:$}",
            2,
        ),
    ] {
        let (_, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), 1);
        let slot = reports.slot_uses.values().next().unwrap().1;
        let Sources::Candidates(values) =
            &reports.results[&slot.block].1.slots.as_ref().unwrap()[0]
        else {
            panic!()
        };
        assert_eq!(values.len(), count);
    }
}
