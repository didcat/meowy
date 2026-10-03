use super::{super::tests::checked, *};

pub(super) fn prepared() -> (Checker, Reports, PointId, PointId) {
    let (mut checker, reports) = checked("r<{n<int32>}>:(({->n:1}));v:r.n");
    let input = checker
        .operations
        .values()
        .filter_map(|op| op.input)
        .find(|input| checker.coercions.contains_key(input))
        .unwrap();
    let anchor = checker
        .grouped_consumer(&reports, input, 0, Span::default())
        .unwrap()
        .unwrap();
    (checker, reports, input, anchor)
}

pub(super) fn state(checker: &Checker, reports: &Reports) -> String {
    format!(
        "{reports:?}{:?}{:?}{:?}{:?}{:?}{:?}",
        checker.points,
        checker.group_inputs,
        checker.region_edges,
        checker.coercions,
        checker.bodies,
        checker.edge_counts()
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
pub(crate) fn mixed_consumers_reject_ambiguous_markers_without_observed_coercions() {
    for anchor_target in [false, true] {
        for observed in [false, true] {
            let (mut checker, mut reports, input, anchor) = prepared();
            let op = checker.coercions[&input].clone();
            let target = if anchor_target { anchor } else { op.input };
            checker.coercions.insert(target, op);
            if observed {
                reports
                    .effects
                    .insert(target, reports.effects[&input].clone());
            } else {
                reports.effects.remove(&target);
            }
            rejected(&mut checker, &reports, input);
            rejected(&mut checker, &reports, target);
        }
    }
}

#[test]
pub(crate) fn mixed_consumers_reject_late_identity_corruption_atomically() {
    for fault in 0..8 {
        let (mut checker, mut reports, input, anchor) = prepared();
        let group = checker.coercions[&input].input;
        let inner = checker.group_inputs[&group].input;
        let block = checker.points[input].block.unwrap();
        match fault {
            0 => checker.group_inputs.get_mut(&group).unwrap().owner += 1,
            1 => {
                checker.region_edges.remove(&group);
            }
            2 => checker.bodies.get_mut(&block).unwrap().owner += 1,
            3 => checker.group_inputs.get_mut(&group).unwrap().block = None,
            4 => checker.coercions.get_mut(&inner).unwrap().owner += 1,
            5 => {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&inner).unwrap() else {
                    panic!()
                };
                op.control = !op.control;
            }
            6 => {
                checker.bodies.remove(&block);
            }
            7 => checker.points[anchor].owner += 1,
            _ => unreachable!(),
        }
        rejected(&mut checker, &reports, input);
    }
}

#[test]
pub(crate) fn mixed_consumers_reject_consistent_group_forward_cycles() {
    let (mut checker, reports, input, _) = prepared();
    let group = checker.coercions[&input].input;
    let span = checker.points[input].span;
    checker.points[input].parent = Some(group);
    checker.points[group].span = span;
    let marker = checker.group_inputs.get_mut(&group).unwrap();
    marker.input = input;
    marker.span = span;
    checker.region_edges.insert(
        group,
        [
            Edge::new(Port::Entry(group), Port::Entry(input), Route::Next),
            Edge::new(Port::Normal(input), Port::Normal(group), Route::Next),
        ],
    );
    assert_eq!(
        checker
            .forward_coercion_input(&reports, input, 0, Span::default())
            .unwrap(),
        Some(group)
    );
    for start in [input, group] {
        rejected(&mut checker, &reports, start);
    }
}

#[test]
pub(crate) fn mixed_consumers_share_exact_hop_and_work_bounds_without_payload() {
    let (mut checker, mut reports, input, anchor) = prepared();
    reports.parts = 0;
    let before = state(&checker, &reports);
    let mut cursor = input;
    let mut kinds = Vec::new();
    while cursor != anchor {
        if let Some(op) = checker.coercions.get(&cursor) {
            kinds.push(true);
            cursor = op.input;
        } else {
            kinds.push(false);
            cursor = checker.group_inputs[&cursor].input;
        }
    }
    assert_eq!(kinds, [true, false, true, false, true]);
    for (start, limit, allowed) in [(anchor, 0, true), (input, 5, true), (input, 4, false)] {
        let result = checker.grouped_consumer_limited(&reports, start, 0, Span::default(), limit);
        assert_eq!(result.is_ok(), allowed);
        if let Ok(result) = result {
            assert_eq!(result, Some(anchor));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(state(&checker, &reports), before);
    }
    let work = checker.flow.work;
    checker
        .grouped_consumer_limited(&reports, input, 0, Span::default(), 5)
        .unwrap();
    let work = checker.flow.work - work;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.grouped_consumer_limited(&reports, input, 0, Span::default(), 5);
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
pub(crate) fn mixed_consumers_keep_unobserved_results_and_logical_terminals_opaque() {
    for missing in [false, true] {
        let (mut checker, mut reports, input, _) = prepared();
        let group = checker.coercions[&input].input;
        let inner = checker.group_inputs[&group].input;
        if missing {
            reports.effects.remove(&inner);
        } else {
            let (_, Effect::Coercion(op)) = reports.effects.get_mut(&inner).unwrap() else {
                panic!()
            };
            op.result = false;
        }
        let before = state(&checker, &reports);
        assert!(
            checker
                .grouped_consumer(&reports, input, 0, Span::default())
                .unwrap()
                .is_none()
        );
        assert_eq!(state(&checker, &reports), before);
    }
    let (mut checker, reports) = checked("a<boolean>:((false&&true));b<boolean>:((true||false))");
    let roots = checker
        .operations
        .values()
        .filter_map(|op| op.input)
        .collect::<Vec<_>>();
    let before = state(&checker, &reports);
    assert_eq!(roots.len(), 2);
    for (input, kind) in roots.into_iter().zip([PointKind::And, PointKind::Or]) {
        let mut cursor = input;
        while let Some(next) = checker
            .coercions
            .get(&cursor)
            .map(|op| op.input)
            .or_else(|| checker.group_inputs.get(&cursor).map(|group| group.input))
        {
            cursor = next;
        }
        assert_eq!(checker.points[cursor].kind, kind);
        assert!(
            checker
                .grouped_consumer(&reports, input, 0, Span::default())
                .unwrap()
                .is_none()
        );
    }
    assert!(reports.slot_uses.is_empty());
    assert_eq!(state(&checker, &reports), before);
}
