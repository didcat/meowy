use super::*;
use crate::check::dependencies::ScalarKind;

pub(self) const SOURCE: &str = "a:3.{->$;->tag:true};b:4.{->$;->tag:false};good:a+1;bad:a+b";

#[test]
pub(crate) fn dispatch_binary_primaries_reject_late_operand_type_plan_and_source_faults() {
    for fault in 0..8 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.binaries.keys().last().unwrap();
        let block = checker.dispatch_ops.values().last().unwrap().block;
        match fault {
            0 => reports.effects.get_mut(&id).unwrap().0 += 1,
            1 => {
                let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.inputs.swap(0, 1);
            }
            2 => {
                let ty = BinaryClass::Scalar(ScalarKind::Int {
                    bits: 8,
                    signed: true,
                });
                let captured = checker.binaries.get_mut(&id).unwrap();
                captured.types.inputs = [ty; 2];
                captured.types.result = ty;
                let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.types = captured.types;
            }
            3 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                slots[0].shape = Shape::Scalar(ScalarKind::Bool);
            }
            4 => reports.results.get_mut(&block).unwrap().1.dispatch = None,
            5 => checker.bodies.get_mut(&block).unwrap().owner += 1,
            6 => {
                let op = checker.binaries.get_mut(&id).unwrap();
                op.plan.primary = [false; 2];
                let (_, Effect::Binary(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.plan = op.plan;
            }
            7 => {
                let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.projected = [false; 2];
                op.operation = false;
                op.result = false;
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
pub(crate) fn dispatch_binary_primaries_bound_exact_map_and_shared_work_without_payload() {
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
pub(crate) fn seeded_dispatch_binary_primaries_reject_cycles_in_wrapped_record_copies() {
    let (mut checker, mut reports) = checked("r:3.{->$;->tag:true};copy:((r));x:copy+1");
    let copy = checker.local_reads.values().last().unwrap().local;
    let read = *checker
        .local_reads
        .iter()
        .find(|(_, read)| !reports.receivers.contains_key(&read.local))
        .unwrap()
        .0;
    let op = checker.local_reads.get_mut(&read).unwrap();
    op.local = copy;
    op.storage = copy;
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&read).unwrap() else {
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
pub(crate) fn dispatch_binary_primaries_keep_consumer_and_dispatch_control_independent() {
    for consumer in [false, true] {
        for source in [false, true] {
            let (mut checker, mut reports) = checked(SOURCE);
            for (&id, op) in &mut checker.binaries {
                op.control = consumer;
                let (_, Effect::Binary(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.control = consumer;
            }
            for (&id, op) in &mut checker.dispatch_ops {
                op.control = source;
                let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.control = source;
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
