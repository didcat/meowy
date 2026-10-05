use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "d:@\"debug\";r:3.{->$;->tag:true};good:-r;d.print(\"a{r}b{r}\")";

#[test]
pub(crate) fn dispatch_output_sources_reject_late_shape_origin_owner_and_registration_faults() {
    for fault in 0..10 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.outputs.keys().next().unwrap();
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
                checker.outputs.get_mut(&id).unwrap().parts[3]
                    .as_mut()
                    .unwrap()
                    .source = source;
                let (_, Effect::Output(output)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                output
                    .parts
                    .get_mut(&3)
                    .unwrap()
                    .input
                    .as_mut()
                    .unwrap()
                    .source = source;
            }
            6 => reports.results.get_mut(&block).unwrap().1.dispatch = None,
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
            "{fault}"
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

#[test]
pub(crate) fn dispatch_output_sources_deduplicate_visits_and_share_exact_map_and_work_limits() {
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
        checker.outputs,
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
            checker.outputs,
            checker.edge_counts()
        ),
        before
    );
}

#[test]
pub(crate) fn seeded_dispatch_output_sources_reject_cycles_across_wrapped_record_copies() {
    let (mut checker, mut reports) =
        checked("d:@\"debug\";r:3.{->$;->tag:true};copy:((r));d.print(copy)");
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
pub(crate) fn dispatch_output_sources_preserve_producer_and_consumer_control_independently() {
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
            for (&id, op) in &mut checker.outputs {
                op.control = consumer;
                let (_, Effect::Output(observed)) = reports.effects.get_mut(&id).unwrap() else {
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
