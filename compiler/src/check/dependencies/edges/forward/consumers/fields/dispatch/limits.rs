use super::*;
use crate::check::dependencies::edges::forward::consumers::field_results::lookup::Lookup;

#[test]
pub(crate) fn record_dispatch_fields_share_exact_slot_capacity_and_work_without_payload() {
    let (mut checker, mut reports) = checked("r:3.{->n:1;->z:2};a:r.n;b:r.z");
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
        assert_eq!(result.is_ok(), short == 0);
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn record_dispatch_fields_bound_unique_field_cache_payload_hops_and_work() {
    let (mut checker, reports) = checked("r:3.{->n:1};out:{->r.n;->again:r.n}");
    let sources: Vec<_> = reports
        .direct_sources
        .values()
        .filter(|(_, direct)| direct.source.is_some())
        .map(|(_, direct)| *direct)
        .collect();
    assert_eq!(sources.len(), 2);
    assert_ne!(
        sources[0].source.unwrap().field,
        sources[1].source.unwrap().field
    );
    assert_eq!(
        sources[0].source.unwrap().slot,
        sources[1].source.unwrap().slot
    );
    let roots = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut ctx = Lookup::new(&reports, roots + 2);
    for (direct, remaining) in [(sources[0], 1), (sources[0], 1), (sources[1], 0)] {
        assert_eq!(
            checker
                .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), 1)
                .unwrap(),
            direct.source
        );
        assert_eq!(ctx.parts, remaining);
    }
    let direct = sources[0];
    for (hops, parts) in [(0, roots + 1), (1, roots)] {
        assert!(
            checker
                .field_narrowing_source(
                    &mut Lookup::new(&reports, parts),
                    direct.point,
                    0,
                    Span::default(),
                    hops
                )
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    let start = checker.flow.work;
    checker
        .field_narrowing_source(
            &mut Lookup::new(&reports, roots + 1),
            direct.point,
            0,
            Span::default(),
            1,
        )
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.field_narrowing_source(
            &mut Lookup::new(&reports, roots + 1),
            direct.point,
            0,
            Span::default(),
            1,
        );
        assert_eq!(result.is_ok(), short == 0);
        if short == 0 {
            assert_eq!(result.unwrap(), direct.source);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
