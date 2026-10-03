use super::{super::tests::checked, *};

pub(super) fn prepared() -> (Checker, Reports, PointId, PointId, PointId) {
    let (checker, reports) = checked("r<{n<int32>}>:(({->n:1}));v:((r)).n");
    let input = checker.fields.first_key_value().unwrap().1.input;
    let read = *checker.local_reads.first_key_value().unwrap().0;
    assert_eq!(reports.consumers.len(), 1);
    let anchor = *reports.consumers.first_key_value().unwrap().0;
    (checker, reports, input, anchor, read)
}

pub(super) fn state(checker: &Checker, reports: &Reports) -> String {
    format!(
        "{}{:?}{:?}{:?}",
        super::narrow_limits::state(checker, reports),
        checker.local_reads,
        checker.operations,
        checker.sites
    )
}

pub(super) fn rejected(checker: &mut Checker, reports: &Reports, input: PointId) {
    let before = state(checker, reports);
    let error = checker
        .grouped_consumer(reports, input, 0, Span::default())
        .unwrap_err();
    assert!(error.message.contains("identity"), "{error:?}");
    assert_eq!(state(checker, reports), before);
}

#[test]
pub(crate) fn read_consumers_reject_overlapping_producers_without_read_observations() {
    for kind in 0..4 {
        for observed in [false, true] {
            let (mut checker, mut reports, input, anchor, read) = prepared();
            let op = checker.local_reads[&read].clone();
            let target = match kind {
                0 => input,
                1 => reports.initializers[&op.local].input.unwrap(),
                2 => checker.points[read].parent.unwrap(),
                _ => anchor,
            };
            checker.local_reads.insert(target, op);
            if observed {
                reports
                    .effects
                    .insert(target, reports.effects[&read].clone());
            } else {
                reports.effects.remove(&target);
            }
            rejected(&mut checker, &reports, input);
            rejected(&mut checker, &reports, target);
        }
    }
}

#[test]
pub(crate) fn read_consumers_reject_consistent_cycles_across_initializer_jumps() {
    let (mut checker, mut reports) = checked("a:{->n:1};b:((a));v:b.n");
    let (&read, _) = checker.local_reads.first_key_value().unwrap();
    let local = checker.local_reads.last_key_value().unwrap().1.local;
    let root = reports.initializers[&local].input.unwrap();
    let input = checker.fields.first_key_value().unwrap().1.input;
    let op = checker.local_reads.get_mut(&read).unwrap();
    op.local = local;
    op.storage = local;
    let (
        _,
        Effect::Read {
            local: id, storage, ..
        },
    ) = reports.effects.get_mut(&read).unwrap()
    else {
        panic!()
    };
    *id = local;
    *storage = local;
    assert_eq!(
        checker
            .read_initializer_input(&reports, read, 0, Span::default())
            .unwrap(),
        Some(root)
    );
    assert_ne!(checker.points[root].parent, Some(read));
    for start in [input, root, read] {
        rejected(&mut checker, &reports, start);
    }
}

#[test]
pub(crate) fn read_consumers_share_exact_mixed_hop_and_work_limits_without_payload() {
    let (mut checker, mut reports, input, anchor, _) = prepared();
    reports.parts = 0;
    let before = state(&checker, &reports);
    for (start, limit, allowed) in [(anchor, 0, true), (input, 9, true), (input, 8, false)] {
        let result = checker.grouped_consumer_limited(&reports, start, 0, Span::default(), limit);
        assert_eq!(result.is_ok(), allowed, "{limit}: {result:?}");
        if let Ok(result) = result {
            assert_eq!(result, Some(anchor));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(state(&checker, &reports), before);
    }
    let work = checker.flow.work;
    checker
        .grouped_consumer_limited(&reports, input, 0, Span::default(), 9)
        .unwrap();
    let work = checker.flow.work - work;
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.grouped_consumer_limited(&reports, input, 0, Span::default(), 9);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(result) = result {
            assert_eq!(result, Some(anchor));
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(state(&checker, &reports), before);
    }
}

#[test]
pub(crate) fn read_consumers_keep_missing_evidence_opaque_after_outer_wrappers() {
    for missing in 0..3 {
        let (mut checker, mut reports, input, _, read) = prepared();
        let local = checker.local_reads[&read].local;
        match missing {
            0 => {
                reports.effects.remove(&read);
            }
            1 => {
                reports.initializers.remove(&local);
            }
            2 => {
                let op = checker.local_reads.get_mut(&read).unwrap();
                op.normal = false;
                op.edges.pop();
                let (_, Effect::Read { normal, .. }) = reports.effects.get_mut(&read).unwrap()
                else {
                    panic!()
                };
                *normal = false;
            }
            _ => unreachable!(),
        }
        let before = state(&checker, &reports);
        assert_eq!(
            checker
                .grouped_consumer(&reports, input, 0, Span::default())
                .unwrap(),
            None
        );
        assert_eq!(state(&checker, &reports), before);
    }
}
