use super::*;
use crate::check::dependencies::edges::forward::{
    consumers::field_results::lookup::Lookup, results::inputs::graph::Visit,
};

pub(super) const SOURCE: &str = "r:{->n:1;->m:2};out:r.{->$.n;->saved:$.m}";

#[test]
pub(crate) fn record_receiver_fields_reject_late_source_scope_and_layout_faults() {
    for fault in 0..10 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&dispatch, op) = checker.dispatch_ops.first_key_value().unwrap();
        let local = op.local;
        let id = *checker.fields.keys().last().unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let read = *checker
            .local_reads
            .iter()
            .find(|(_, read)| read.local == local)
            .unwrap()
            .0;
        match fault {
            0 => reports.receivers.get_mut(&local).unwrap().0 += 1,
            1 => checker.points[read].parent = Some(read),
            2 => checker.fields.get_mut(&id).unwrap().count += 1,
            3 => {
                let (_, Effect::Field { input, .. }) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                *input = usize::MAX;
            }
            4 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
            5 | 9 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                if fault == 5 {
                    slots[slot.index].mutable = true;
                } else {
                    slots[slot.index].field = None;
                }
            }
            6 => reports.results.get_mut(&slot.block).unwrap().1.consumer = None,
            7 => {
                checker.proofs.receivers.remove(&local);
            }
            8 => {
                checker.fields.insert(read, checker.fields[&id].clone());
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}{:?}", checker.fields, checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}, {dispatch}"
        );
        assert!(
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .err()
                .unwrap()
                .message
                .contains("identity")
        );
        assert_eq!(
            format!("{reports:?}{:?}{:?}", checker.fields, checker.edge_counts()),
            before
        );
    }
}

#[test]
pub(crate) fn record_receiver_fields_drop_missing_evidence_and_reject_stale_result_links() {
    for fault in 0..3 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&dispatch, op) = checker.dispatch_ops.first_key_value().unwrap();
        let local = op.local;
        match fault {
            0 => {
                reports.eligible.remove(&local);
            }
            1 => {
                reports.receivers.remove(&local);
            }
            2 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.initialized = false;
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
        assert!(
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .is_err()
        );
        assert_eq!(format!("{reports:?}"), before);
        reports.slot_uses.clear();
        assert!(
            checker
                .field_results(&reports, Span::default())
                .unwrap()
                .0
                .is_empty()
        );
    }
}

#[test]
pub(crate) fn record_receiver_fields_share_exact_map_cache_and_work_limits() {
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
    checker.flow.work = 0;
    checker.flow.full = false;
    let parts = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count()
        + reports.field_results.len();
    for short in [0, 1] {
        let mut ctx = Lookup::new(&reports, parts - short);
        for (index, (&id, op)) in checker.fields.clone().iter().enumerate() {
            let result = checker.field_result_source(&mut ctx, id, op.owner, Span::default());
            if short == 1 && index + 1 == reports.field_results.len() {
                assert!(result.unwrap_err().message.contains("budget"));
            } else {
                assert_eq!(
                    result.unwrap(),
                    Some(reports.field_results[&Port::Normal(id)].1)
                );
                assert!(
                    checker
                        .field_result_source(&mut ctx, id, op.owner, Span::default())
                        .unwrap()
                        .is_some()
                );
            }
        }
        assert_eq!(ctx.parts, 0);
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn record_receiver_fields_preserve_control_and_expanded_sources_independently() {
    for receiver in [false, true] {
        for field in [false, true] {
            let (mut checker, mut reports) = checked(SOURCE);
            for (&id, op) in &mut checker.dispatch_ops {
                op.control = receiver;
                let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.control = receiver;
            }
            for (&id, op) in &mut checker.fields {
                op.control = field;
                let (_, Effect::Field { control, .. }) = reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *control = field;
            }
            assert_eq!(
                reports
                    .direct_sources
                    .values()
                    .filter(|(_, direct)| direct.source.is_some())
                    .count(),
                2
            );
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
            assert_eq!(
                checker.field_results(&reports, Span::default()).unwrap().0,
                reports.field_results
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
