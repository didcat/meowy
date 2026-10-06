use super::*;
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn dispatch_list_sources_reject_late_shape_origin_owner_and_registration_faults() {
    for fault in 0..10 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.lists.keys().next().unwrap();
        let block = checker.dispatch_ops.values().next().unwrap().block;
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
            6 => reports.results.get_mut(&block).unwrap().1.dispatch = None,
            7 => {
                reports.index.operations.remove(&id);
            }
            8 => checker.points[checker.list_inputs[&id][3].point].owner += 1,
            9 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                slots[0].mutable = true;
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
            "{fault}"
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

#[test]
pub(crate) fn dispatch_list_sources_deduplicate_visits_and_share_exact_map_and_work_limits() {
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
    let before = format!(
        "{reports:?}{:?}{:?}",
        checker.list_inputs,
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
            checker.list_inputs,
            checker.edge_counts()
        ),
        before
    );
}

#[test]
pub(crate) fn seeded_dispatch_list_sources_reject_cycles_across_wrapped_record_copies() {
    let source = "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:3.{->$;->tag:true};copy:((r));|v<boolean>|xs<T[2]><U[2]>:[copy,v]}";
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
pub(crate) fn dispatch_list_sources_preserve_producer_and_consumer_control_independently() {
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
            for (&id, op) in &mut checker.lists {
                op.control = consumer;
                let (_, Effect::List(observed)) = reports.effects.get_mut(&id).unwrap() else {
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
