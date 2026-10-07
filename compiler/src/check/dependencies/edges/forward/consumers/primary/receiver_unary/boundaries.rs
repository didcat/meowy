use super::*;
use crate::check::dependencies::TypedKind;

pub(super) const SOURCE: &str = "b:@\"bits\";<R>:<{-><int32>;tag<boolean>}>;r:3.{->$;->tag:true};out:r.{good:b.not($~<R>);bad:b.not($~<R>)}";

#[test]
pub(crate) fn receiver_unary_sources_reject_late_types_layouts_and_ascription_faults() {
    for init in ["{->3;->tag:true}", "3.{->$;->tag:true}"] {
        for fault in 0..16 {
            let source = SOURCE.replace("3.{->$;->tag:true}", init);
            let (mut checker, mut reports) = checked(&source);
            let id = *checker.unaries.keys().last().unwrap();
            let ascription = *checker.typed_ops.keys().last().unwrap();
            let slot = reports.slot_uses[&Port::Projection { point: id, step: 0 }].1;
            match fault {
                0 => reports.effects.get_mut(&id).unwrap().0 += 1,
                1 => {
                    let (_, Effect::Unary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.input = usize::MAX;
                }
                2..=4 => {
                    let bits = if fault == 2 {
                        8
                    } else if fault == 4 {
                        7
                    } else {
                        32
                    };
                    let signed = fault != 3;
                    checker.unaries.get_mut(&id).unwrap().ty = hir::Type::Int { bits, signed };
                    let (_, Effect::Unary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.ty = ScalarKind::Int { bits, signed };
                }
                5 => {
                    checker.unaries.get_mut(&id).unwrap().primary = false;
                    let (_, Effect::Unary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.primary = false;
                }
                6 => checker.unaries.get_mut(&id).unwrap().edges.clear(),
                7 => {
                    let row = &mut reports.results.get_mut(&slot.block).unwrap().1;
                    row.consumer = None;
                    row.dispatch = None;
                }
                8 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
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
                12 => {
                    reports.index.operations.remove(&id);
                }
                13 | 15 => {
                    let (_, Effect::Typed(op)) = reports.effects.get_mut(&ascription).unwrap()
                    else {
                        panic!()
                    };
                    if fault == 13 {
                        op.input = usize::MAX;
                    } else {
                        op.op = TypedKind::Predicate;
                    }
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
                checker.unaries,
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
                    checker.unaries,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}

#[test]
pub(crate) fn receiver_unary_sources_preserve_missing_evidence_and_exact_map_work_limits() {
    for missing in 0..3 {
        let (mut checker, mut reports) = checked(SOURCE);
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
                for id in checker.typed_ops.keys() {
                    reports.effects.remove(id);
                }
            }
            _ => unreachable!(),
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
pub(crate) fn seeded_receiver_unary_sources_reject_mixed_receiver_initializer_cycles() {
    let (mut checker, mut reports) = checked(
        "<R>:<{-><int32>;tag<boolean>}>;r:3.{->$;->tag:true};out:r.{alias:(($));x:-(alias~<R>)}",
    );
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
pub(crate) fn receiver_unary_sources_keep_source_receiver_ascription_and_unary_control_independent()
{
    for source in [false, true] {
        for receiver in [false, true] {
            for ascription in [false, true] {
                for unary in [false, true] {
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
                    for (&id, op) in &mut checker.typed_ops {
                        op.control = ascription;
                        let (_, Effect::Typed(observed)) = reports.effects.get_mut(&id).unwrap()
                        else {
                            panic!()
                        };
                        observed.control = ascription;
                    }
                    for (&id, op) in &mut checker.unaries {
                        op.control = unary;
                        let (_, Effect::Unary(observed)) = reports.effects.get_mut(&id).unwrap()
                        else {
                            panic!()
                        };
                        observed.control = unary;
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
}
