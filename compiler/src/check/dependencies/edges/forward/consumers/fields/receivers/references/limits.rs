use super::{boundaries::SOURCE, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;

#[test]
pub(crate) fn shared_field_links_keep_source_receiver_field_control_and_walks_independent() {
    let (mut checker, mut reports) = checked(SOURCE);
    assert_eq!(
        reports
            .direct_sources
            .values()
            .filter(|(_, direct)| direct.source.is_some())
            .count(),
        2
    );
    for source in [false, true] {
        for receiver in [false, true] {
            for field in [false, true] {
                for (&id, op) in &mut checker.dispatch_ops {
                    op.control = if op.shared_primary.is_some() {
                        receiver
                    } else {
                        source
                    };
                    let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    observed.control = op.control;
                }
                for (&id, op) in &mut checker.fields {
                    op.control = field;
                    let (_, Effect::Field { control, .. }) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    *control = field;
                }
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
}

#[test]
pub(crate) fn shared_field_links_bound_exact_operation_maps_result_roots_and_work() {
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
    checker.flow.work = 0;
    checker.flow.full = false;
    reports.parts = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let expected = checker.field_results(&reports, Span::default()).unwrap();
    let work = checker.flow.work - start;
    assert_eq!(expected, (reports.field_results.clone(), 0));
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.field_results(&reports, Span::default());
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    checker.flow.work = 0;
    checker.flow.full = false;
    reports.parts -= 1;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .field_results(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
