use super::*;
use crate::check::dependencies::edges::forward::results::tests::checked;

#[test]
pub(crate) fn dispatch_source_index_and_histories_share_exact_capacity_payload_and_work() {
    let (mut checker, reports) = checked("a:3.{->$};b:4.{}");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut parts = 2;
    let work = checker.flow.work;
    let expected = checker
        .dispatch_result_index(&reports, &mut parts, 2, Span::default())
        .unwrap();
    let work = checker.flow.work - work;
    assert_eq!((expected.len(), parts), (2, 0));
    for (limit, room) in [(1, 2), (2, 1)] {
        let mut parts = room;
        assert!(
            checker
                .dispatch_result_index(&reports, &mut parts, limit, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(parts, room);
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let mut parts = 2;
        let result = checker.dispatch_result_index(&reports, &mut parts, 2, Span::default());
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
            assert_eq!(parts, 0);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
            assert_eq!(parts, 2);
        }
    }
    checker.flow.work = 0;
    checker.flow.full = false;
    let limit = reports.effects.len() + reports.blocks.len() + reports.results.len();
    assert_eq!(
        checker
            .result_sources_limited(&reports, Span::default(), limit, 8)
            .unwrap(),
        (reports.results.clone(), 0)
    );
    for (limit, parts) in [(limit - 1, 8), (limit, 7)] {
        assert!(
            checker
                .result_sources_limited(&reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn dispatch_source_index_rejects_late_conflicts_without_publishing_or_spending_parts() {
    for fault in 0..4 {
        let (mut checker, mut reports) = checked("a:3.{->$};b:4.{->$}");
        let (&id, op) = checker.dispatch_ops.last_key_value().unwrap();
        let block = op.block;
        match fault {
            0 => reports.effects.get_mut(&id).unwrap().0 += 1,
            1 => checker.bodies.get_mut(&block).unwrap().span.end = 0,
            2 => checker.bodies.get_mut(&block).unwrap().parent = None,
            3 => {
                reports
                    .blocks
                    .insert(block, *reports.blocks.values().next().unwrap());
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut parts = 2;
        assert!(
            checker
                .dispatch_result_index(&reports, &mut parts, 2, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(parts, 2);
        assert!(
            checker
                .result_sources(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
