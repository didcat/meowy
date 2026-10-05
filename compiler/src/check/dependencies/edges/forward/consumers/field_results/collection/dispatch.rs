use super::{super::super::tests::checked, *};

#[test]
pub(crate) fn record_dispatch_field_results_share_map_root_payload_and_work_limits() {
    let (mut checker, reports) = checked("r:3.{->n:1;->z:2};a:r.n;b:r.z");
    let expected = &reports.field_results;
    assert_eq!(expected.len(), 2);
    let limit = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + reports.slot_uses.len()
        + reports.candidate_inputs.len()
        + expected.len();
    let roots = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .field_results_limited(&reports, Span::default(), limit, roots)
            .unwrap(),
        (expected.clone(), 0)
    );
    let work = checker.flow.work - start;
    for (limit, parts) in [(limit - 1, roots), (limit, roots - 1)] {
        assert!(
            checker
                .field_results_limited(&reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.field_results_limited(&reports, Span::default(), limit, roots);
        assert_eq!(result.is_ok(), short == 0);
        if short == 0 {
            assert_eq!(result.unwrap(), (expected.clone(), 0));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
