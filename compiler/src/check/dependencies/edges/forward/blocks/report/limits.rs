use super::{super::tests::checked, *};
use crate::check::dependencies::bodies::completion::Shape;

#[test]
pub(crate) fn block_reports_share_exact_map_and_work_limits_across_duplicate_visits() {
    for stage in 0..3 {
        let (mut checker, mut reports) = checked("v:{->1};w:{->2};f<int32>:(){->3}");
        let effects = reports.effects.clone();
        let mut expected = reports.blocks.clone();
        for (_, observed) in expected.values_mut() {
            observed.normal = stage != 2;
            observed.result = stage != 1;
        }
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.retain(|port| match port {
                Port::BlockNormal(_) => stage != 2,
                Port::BlockResult(_) => stage != 1,
                _ => false,
            });
            walk.ports.extend(walk.ports.clone());
        }
        let stored = reports.blocks.clone();
        let counts = checker.edge_counts();
        let limit = effects.len() + expected.len();
        let before = checker.flow.work;
        assert_eq!(
            checker
                .block_effects_limited(&reports, Span::default(), limit)
                .unwrap(),
            expected
        );
        let work = checker.flow.work - before;
        for room in [effects.len() - 1, effects.len(), limit - 1] {
            assert!(
                checker
                    .block_effects_limited(&reports, Span::default(), room)
                    .unwrap_err()
                    .message
                    .contains("budget")
            );
        }
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            let result = checker.block_effects_limited(&reports, Span::default(), limit);
            assert_eq!(result.is_ok(), short == 0);
            if let Ok(actual) = result {
                assert_eq!(actual, expected);
                assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
            }
        }
        assert_eq!(reports.effects, effects);
        assert_eq!(reports.blocks, stored);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn block_reports_merge_atomically_on_metadata_conflicts_and_exhaustion() {
    let (mut checker, reports) = checked("v:{->1}");
    let id = reports.entries[&0].0;
    let mut blocks = Blocks::new();
    assert!(
        checker
            .record_block_effect(0, Port::BlockNormal(id), &mut blocks, 0, Span::default())
            .is_err()
    );
    assert!(blocks.is_empty());
    checker
        .record_block_effect(0, Port::BlockNormal(id), &mut blocks, 1, Span::default())
        .unwrap();
    let expected = blocks.clone();
    for fault in 0..6 {
        let mut blocks = expected.clone();
        let (owner, observed) = blocks.get_mut(&id).unwrap();
        match fault {
            0 => *owner = 1,
            1 => observed.parent = Some(0),
            2 => observed.span.start += 1,
            3 => observed.span.end += 1,
            4 => observed.completion.normal = false,
            5 => observed.completion.result = Shape::Other,
            _ => unreachable!(),
        }
        let before = blocks.clone();
        assert!(
            checker
                .record_block_effect(0, Port::BlockResult(id), &mut blocks, 1, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(blocks, before);
    }
    checker
        .record_block_effect(0, Port::BlockNormal(id), &mut blocks, 0, Span::default())
        .unwrap();
    assert_eq!(blocks, expected);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_block_effect(0, Port::BlockResult(id), &mut blocks, 1, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(blocks, expected);
}

#[test]
pub(crate) fn block_reports_discard_a_valid_prefix_when_a_later_block_is_invalid() {
    let (mut checker, mut reports) = checked("a:{->1};b:{->2}");
    let ids: Vec<_> = checker
        .bodies
        .iter()
        .filter_map(|(&id, body)| body.parent.map(|_| id))
        .collect();
    let first = ids[0];
    let last = ids[1];
    reports.entries.get_mut(&0).unwrap().1.ports = vec![
        Port::BlockNormal(first),
        Port::BlockResult(first),
        Port::BlockNormal(last),
    ];
    let blocks = reports.blocks.clone();
    let effects = reports.effects.clone();
    let counts = checker.edge_counts();
    let parent = checker.bodies.get_mut(&last).unwrap().parent.take();
    assert!(
        checker
            .block_effects_limited(&reports, Span::default(), effects.len() + 2)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(reports.blocks, blocks);
    assert_eq!(reports.effects, effects);
    assert_eq!(checker.edge_counts(), counts);
    checker.bodies.get_mut(&last).unwrap().parent = parent;
    let actual = checker
        .block_effects_limited(&reports, Span::default(), effects.len() + 2)
        .unwrap();
    assert_eq!(actual.len(), 2);
    assert!(actual[&first].1.normal && actual[&first].1.result);
    assert!(actual[&last].1.normal && !actual[&last].1.result);
    assert_eq!(reports.blocks, blocks);
}
