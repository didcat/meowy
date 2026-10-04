use super::{super::tests::checked, *};

pub(super) fn prepared() -> (Checker, Reports, PointId, PointId, PointId) {
    let (checker, reports) = checked("r<{n<int32>}>:(({->n:1}~<{n<int32>}>));v:((r)).n");
    let input = checker.fields.first_key_value().unwrap().1.input;
    let anchor = *reports.consumers.first_key_value().unwrap().0;
    let id = *checker.typed_ops.first_key_value().unwrap().0;
    (checker, reports, input, anchor, id)
}

pub(super) fn state(checker: &Checker, reports: &Reports) -> String {
    format!(
        "{}{:?}",
        super::read_limits::state(checker, reports),
        checker.typed_ops
    )
}

#[test]
pub(crate) fn ascription_consumers_share_exact_mixed_hop_work_and_map_limits() {
    let (mut checker, mut reports, input, anchor, _) = prepared();
    reports.parts = 0;
    let before = state(&checker, &reports);
    for (start, limit, allowed) in [(anchor, 0, true), (input, 10, true), (input, 9, false)] {
        let result = checker.grouped_consumer_limited(&reports, start, 0, Span::default(), limit);
        assert_eq!(result.is_ok(), allowed, "{limit}: {result:?}");
        if let Ok(result) = result {
            assert_eq!(result, Some(anchor));
        }
        assert_eq!(state(&checker, &reports), before);
    }
    let limit = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + reports.slot_uses.len();
    let work = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        reports.slot_uses
    );
    let work = checker.flow.work - work;
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
        if let Ok(result) = result {
            assert_eq!(result, reports.slot_uses);
        }
        assert_eq!(state(&checker, &reports), before);
    }
}

#[test]
pub(crate) fn ascription_consumers_reject_every_overlapping_producer_without_observations() {
    for kind in 0..5 {
        for observed in [false, true] {
            let (mut checker, mut reports, input, anchor, id) = prepared();
            let target = match kind {
                0 => *checker.group_inputs.first_key_value().unwrap().0,
                1 => *checker.coercions.first_key_value().unwrap().0,
                2 => {
                    *checker
                        .narrowings
                        .iter()
                        .find(|(_, op)| checker.local_reads.contains_key(&op.input))
                        .unwrap()
                        .0
                }
                3 => *checker.local_reads.first_key_value().unwrap().0,
                _ => anchor,
            };
            checker
                .typed_ops
                .insert(target, checker.typed_ops[&id].clone());
            if observed {
                reports.effects.insert(target, reports.effects[&id].clone());
            } else {
                reports.effects.remove(&target);
            }
            let before = state(&checker, &reports);
            for start in [input, target] {
                let error = checker
                    .grouped_consumer(&reports, start, 0, Span::default())
                    .unwrap_err();
                assert!(error.message.contains("identity"));
                assert_eq!(state(&checker, &reports), before);
            }
        }
    }
}

#[test]
pub(crate) fn ascription_consumers_reject_consistent_group_cycles() {
    let (mut checker, mut reports) = checked("v:(({->n:1})~<{n<int32>}>).n");
    let (&id, op) = checker.typed_ops.first_key_value().unwrap();
    let group = op.input;
    let span = op.span;
    checker.points[id].parent = Some(group);
    checker.points[group].span = span;
    let marker = checker.group_inputs.get_mut(&group).unwrap();
    marker.input = id;
    marker.span = span;
    checker.region_edges.insert(
        group,
        [
            Edge::new(Port::Entry(group), Port::Entry(id), Route::Next),
            Edge::new(Port::Normal(id), Port::Normal(group), Route::Next),
        ],
    );
    reports.parts = 0;
    assert_eq!(
        checker
            .unchanged_ascription_input(&reports, id, 0, span)
            .unwrap(),
        Some(group)
    );
    let before = state(&checker, &reports);
    for start in [id, group] {
        let error = checker
            .grouped_consumer(&reports, start, 0, span)
            .unwrap_err();
        assert!(error.message.contains("identity"));
        assert_eq!(state(&checker, &reports), before);
    }
}

#[test]
pub(crate) fn ascription_consumers_discard_late_report_and_stage_failures() {
    for fault in 0..5 {
        let (mut checker, mut reports) = checked("a:{->n:0}.n;v:({->n:1}~<{n<int32>}>).n");
        let id = *checker.typed_ops.first_key_value().unwrap().0;
        match fault {
            0 => {
                let (_, Effect::Typed(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.changed = true;
            }
            1 => checker.typed_ops.get_mut(&id).unwrap().edges[1].route = Route::Checked,
            2 => checker.points[id].complete = false,
            3 => {
                reports.index.operations.remove(&id);
            }
            4 => {
                checker
                    .bodies
                    .get_mut(&checker.points[id].block.unwrap())
                    .unwrap()
                    .owner += 1
            }
            _ => unreachable!(),
        }
        let before = state(&checker, &reports);
        super::super::limits::rejected(&mut checker, &reports);
        assert_eq!(state(&checker, &reports), before);
    }
}

#[test]
pub(crate) fn ascription_consumers_preserve_unknown_slot_values_and_candidate_histories() {
    use super::super::super::results::Sources;
    for source in [
        "r:{->n:=1};v:({->n:=1}~<(r<>)>).n",
        "v:({->inner:{->n:1}}~<{inner<{n<int32>}>}>).inner",
    ] {
        let (_, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), 1);
        let (_, slot) = reports.slot_uses.first_key_value().unwrap().1;
        assert_eq!(
            reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index],
            Sources::Unknown
        );
    }
    let (mut checker, mut reports) =
        checked("flag:=false;r:{|flag|->n:1;|!flag|->n:2};v:(r~<{n<int32>}>).n");
    let (_, slot) = *reports.slot_uses.first_key_value().unwrap().1;
    let Sources::Candidates(candidates) =
        &reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index]
    else {
        panic!()
    };
    assert_eq!(candidates.len(), 2);
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    let before = reports.effects.clone();
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert_eq!(reports.effects, before);
    let before = state(&checker, &reports);
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    assert_eq!(state(&checker, &reports), before);
}
