use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "r:3.{->$;->tag:true};good<int32>:r;bad<int32><null>:r";

#[test]
pub(crate) fn dispatch_coercion_sources_reject_late_type_origin_and_registration_faults() {
    for fault in 0..8 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .coercions
            .iter()
            .rev()
            .find(|(_, op)| op.primary)
            .unwrap()
            .0;
        let block = checker.dispatch_ops.values().next().unwrap().block;
        match fault {
            0..=4 => {
                let source = match fault {
                    0 => None,
                    1 => Some(Shape::Scalar(ScalarKind::Bool)),
                    2 => Some(Shape::Scalar(ScalarKind::Int {
                        bits: 32,
                        signed: false,
                    })),
                    3 => Some(Shape::Never),
                    4 => Some(Shape::Scalar(ScalarKind::Int {
                        bits: 7,
                        signed: true,
                    })),
                    _ => unreachable!(),
                };
                if fault != 0 {
                    checker.coercions.get_mut(&id).unwrap().source = source;
                }
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.source = source;
            }
            5 => reports.results.get_mut(&block).unwrap().1.dispatch = None,
            6 => {
                reports.index.operations.remove(&id);
            }
            7 => {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.input = usize::MAX;
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn dispatch_coercion_sources_share_exact_map_and_work_without_new_payload() {
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
pub(crate) fn seeded_dispatch_coercion_sources_reject_cycles_across_record_initializers() {
    let (mut checker, mut reports) = checked("r:3.{->$;->tag:true};copy:((r));x<int32>:copy");
    let copy = checker.local_reads.values().last().unwrap().local;
    let id = *checker
        .local_reads
        .iter()
        .find(|(_, read)| !reports.receivers.contains_key(&read.local))
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
pub(crate) fn dispatch_coercion_sources_preserve_producer_and_consumer_control_independently() {
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
            for (&id, op) in &mut checker.coercions {
                op.control = consumer;
                let (_, Effect::Coercion(observed)) = reports.effects.get_mut(&id).unwrap() else {
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
