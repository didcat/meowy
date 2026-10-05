use super::super::field_results::lookup::Lookup;
use super::*;

#[test]
pub(crate) fn receiver_sources_share_field_cache_payload_hops_and_work() {
    let (mut checker, reports) = checked("r:{->n:1};v:r.n.{->$;->again:$}");
    let sources: Vec<_> = reports
        .direct_sources
        .values()
        .filter(|(_, direct)| direct.source.is_some())
        .map(|(_, direct)| *direct)
        .collect();
    assert_eq!(sources.len(), 2);
    let parts = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count()
        + 1;
    let direct = sources[0];
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut ctx = Lookup::new(&reports, parts);
    let start = checker.flow.work;
    assert_eq!(
        checker
            .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), 3)
            .unwrap(),
        direct.source
    );
    let work = checker.flow.work - start;
    assert_eq!(ctx.parts, 0);
    assert_eq!(
        checker
            .field_narrowing_source(&mut ctx, sources[1].point, 0, Span::default(), 3)
            .unwrap(),
        direct.source
    );
    assert_eq!(ctx.parts, 0);
    for (hops, parts) in [(2, parts), (3, parts - 1)] {
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
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.field_narrowing_source(
            &mut Lookup::new(&reports, parts),
            direct.point,
            0,
            Span::default(),
            3,
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

#[test]
pub(crate) fn receiver_sources_share_block_wrapper_hops_and_work_without_payload() {
    let (mut checker, mut reports) = checked("v:{->3}.{->$}");
    let direct = reports
        .direct_sources
        .values()
        .find(|(_, direct)| direct.block.is_some())
        .unwrap()
        .1;
    reports.parts = 0;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .grouped_consumer_limited(&reports, direct.point, 0, Span::default(), 2)
            .unwrap(),
        Some(direct.block.unwrap().consumer)
    );
    let work = checker.flow.work - start;
    assert!(
        checker
            .grouped_consumer_limited(&reports, direct.point, 0, Span::default(), 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result =
            checker.grouped_consumer_limited(&reports, direct.point, 0, Span::default(), 2);
        assert_eq!(result.is_ok(), short == 0);
        if short == 1 {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn receiver_sources_preserve_read_and_dispatch_control_independently() {
    for read_control in [false, true] {
        for dispatch_control in [false, true] {
            let (mut checker, mut reports) = checked("v:{->3}.{->$}");
            let (&read, op) = checker.local_reads.first_key_value().unwrap();
            let dispatch = reports.receivers[&op.local].1;
            checker.local_reads.get_mut(&read).unwrap().control = read_control;
            checker.dispatch_ops.get_mut(&dispatch).unwrap().control = dispatch_control;
            let (_, Effect::Read { control, .. }) = reports.effects.get_mut(&read).unwrap() else {
                panic!()
            };
            *control = read_control;
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.control = dispatch_control;
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            assert_eq!(
                checker.direct_sources(&reports, Span::default()).unwrap().0,
                reports.direct_sources
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
}
