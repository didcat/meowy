use super::*;
use crate::check::dependencies::edges::forward::results::{inputs::graph::Visit, tests::checked};

#[test]
pub(crate) fn direct_block_sources_share_exact_map_cache_and_work_limits_with_fields() {
    let (mut checker, reports) =
        checked("r:{->n:1};n:{->r.n};copy<int32>:((n~<int32>));s:{->copy}");
    assert!(
        reports
            .direct_sources
            .values()
            .any(|(_, direct)| direct.source.is_some())
    );
    assert!(
        reports
            .direct_sources
            .values()
            .any(|(_, direct)| direct.block.is_some())
    );
    let limit = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + reports.slot_uses.len()
        + reports.candidate_inputs.len()
        + reports.field_results.len()
        + reports.direct_sources.len();
    let parts = reports.results.len()
        + reports
            .candidate_inputs
            .values()
            .map(|(_, input)| input.candidate.statement)
            .collect::<BTreeSet<_>>()
            .len()
        + reports
            .candidate_walk
            .visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Root(_)))
            .count()
        + 1;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let work = checker.flow.work;
    let expected = checker
        .direct_sources_limited(&reports, Span::default(), limit, parts)
        .unwrap();
    let work = checker.flow.work - work;
    assert_eq!(expected, (reports.direct_sources.clone(), 0));
    for (limit, parts) in [(limit - 1, parts), (limit, parts - 1)] {
        assert!(
            checker
                .direct_sources_limited(&reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let actual = checker.direct_sources_limited(&reports, Span::default(), limit, parts);
        if short == 0 {
            assert_eq!(actual.unwrap(), expected);
        } else {
            assert!(actual.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn direct_block_walk_shares_qualification_and_traversal_budgets_atomically() {
    let (mut checker, reports) = checked("r:{->n:1};n:{->r.n};s:{->n;->echo:((n))}");
    let (_, left) = checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let scratch = MAX_EDGES - left;
    let work = checker.flow.work;
    let expected = checker
        .direct_walk_limited(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - work;
    assert_eq!(expected.0, reports.expanded_walk);
    assert!(MAX_EDGES - expected.1 > scratch + expected.0.visits.len());
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .direct_walk_limited(&reports, Span::default(), scratch)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let actual = checker.direct_walk_limited(&reports, Span::default(), MAX_EDGES);
        if short == 0 {
            assert_eq!(actual.unwrap(), expected);
        } else {
            assert!(actual.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
