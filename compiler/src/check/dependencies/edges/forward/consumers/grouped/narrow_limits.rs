use super::*;
use crate::check::dependencies::{CoercionKind, Narrowing};

pub(super) fn seeded() -> (Checker, Reports, PointId, PointId, PointId) {
    let (mut checker, mut reports, input, anchor) = super::mixed::prepared();
    let group = checker.coercions[&input].input;
    let id = checker.group_inputs[&group].input;
    let op = checker.coercions.remove(&id).unwrap();
    assert_eq!(op.kind, CoercionKind::Forward);
    assert!(!op.primary);
    let mut effect = reports
        .effects
        .values()
        .find(|(_, effect)| matches!(effect, Effect::Narrowing(_)))
        .cloned()
        .unwrap();
    let (owner, Effect::Narrowing(observed)) = &mut effect else {
        panic!()
    };
    *owner = op.owner;
    observed.input = op.input;
    observed.changed = false;
    observed.normal = true;
    observed.control = op.control;
    observed.operation = false;
    observed.result = true;
    reports.effects.insert(id, effect);
    checker.coercion_edges -= op.edges.len();
    checker.narrowing_edges += op.edges.len();
    checker.narrowings.insert(
        id,
        Narrowing {
            owner: op.owner,
            input: op.input,
            changed: false,
            normal: true,
            control: op.control,
            span: op.span,
            edges: op.edges,
        },
    );
    (checker, reports, input, anchor, id)
}

pub(super) fn state(checker: &Checker, reports: &Reports) -> String {
    format!(
        "{}{:?}",
        super::mixed::state(checker, reports),
        checker.narrowings
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
pub(crate) fn seeded_narrow_consumers_share_mixed_hop_work_and_payload_bounds() {
    let (mut checker, mut reports, input, anchor, id) = seeded();
    reports.parts = 0;
    let before = state(&checker, &reports);
    let group = checker.coercions[&input].input;
    assert_eq!(checker.group_inputs[&group].input, id);
    let inner = checker.narrowings[&id].input;
    assert_eq!(
        checker.coercions[&checker.group_inputs[&inner].input].input,
        anchor
    );
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
pub(crate) fn seeded_narrow_consumers_reject_all_producer_overlap_without_observations() {
    for target in 0..3 {
        for observed in [false, true] {
            let (mut checker, mut reports, input, anchor, id) = seeded();
            let target = match target {
                0 => input,
                1 => checker.coercions[&input].input,
                _ => anchor,
            };
            checker
                .narrowings
                .insert(target, checker.narrowings[&id].clone());
            if observed {
                reports.effects.insert(target, reports.effects[&id].clone());
            } else {
                reports.effects.remove(&target);
            }
            rejected(&mut checker, &reports, input);
            rejected(&mut checker, &reports, target);
        }
    }
}

#[test]
pub(crate) fn seeded_narrow_consumers_reject_late_identity_corruption_atomically() {
    for fault in 0..6 {
        let (mut checker, mut reports, input, _, id) = seeded();
        let child = checker.narrowings[&id].input;
        match fault {
            0 => checker.narrowings.get_mut(&id).unwrap().owner += 1,
            1 => checker.narrowings.get_mut(&id).unwrap().span.end += 1,
            2 => checker.narrowings.get_mut(&id).unwrap().edges[1].route = Route::Checked,
            3 => {
                let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.control = !op.control;
            }
            4 => checker.points[child].parent = None,
            5 => checker.points[child].kind = PointKind::And,
            _ => unreachable!(),
        }
        rejected(&mut checker, &reports, input);
    }
}

#[test]
pub(crate) fn seeded_narrow_consumers_reject_consistent_three_wrapper_cycles() {
    let (mut checker, mut reports, input, _, id) = seeded();
    let group = checker.coercions[&input].input;
    let span = checker.points[input].span;
    checker.points[input].parent = Some(id);
    checker.points[group].span = span;
    checker.points[id].span = span;
    checker.group_inputs.get_mut(&group).unwrap().span = span;
    let op = checker.narrowings.get_mut(&id).unwrap();
    op.input = input;
    op.span = span;
    op.edges = vec![
        Edge::new(Port::Entry(id), Port::Entry(input), Route::Next),
        Edge::new(Port::Normal(input), Port::Normal(id), Route::Next),
    ];
    let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.input = input;
    assert_eq!(
        checker
            .unchanged_narrowing_input(&reports, id, 0, span)
            .unwrap(),
        Some(input)
    );
    assert_eq!(
        checker
            .forward_coercion_input(&reports, input, 0, span)
            .unwrap(),
        Some(group)
    );
    for start in [input, group, id] {
        rejected(&mut checker, &reports, start);
    }
}
