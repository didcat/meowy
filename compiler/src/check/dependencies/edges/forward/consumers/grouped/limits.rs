use super::{super::tests::checked, *};

#[test]
pub(crate) fn grouped_consumers_bound_hops_scratch_and_registry_without_order_inference() {
    let depth = 32;
    let source = format!("v:{}{{->n:1}}{}.n", "(".repeat(depth), ")".repeat(depth));
    let (mut checker, reports) = checked(&source);
    let field = checker.fields.values().next().unwrap();
    let (input, owner) = (field.input, field.owner);
    let mut anchor = input;
    for _ in 0..depth {
        anchor = checker.group_inputs[&anchor].input;
    }
    assert!(reports.consumers.contains_key(&anchor));
    for (point, limit, expected) in [
        (anchor, 0, true),
        (input, depth, true),
        (input, depth - 1, false),
        (input, 0, false),
    ] {
        let result =
            checker.grouped_consumer_limited(&reports, point, owner, Span::default(), limit);
        assert_eq!(result.is_ok(), expected);
        if let Ok(result) = result {
            assert_eq!(result, Some(anchor));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    let ids = checker.group_inputs.keys().copied().collect::<Vec<_>>();
    let moved = ids
        .iter()
        .copied()
        .zip(ids.iter().copied().rev())
        .collect::<BTreeMap<_, _>>();
    for index in 0..ids.len() / 2 {
        checker.points.swap(ids[index], ids[ids.len() - index - 1]);
    }
    for point in &mut checker.points {
        point.parent = point.parent.map(|id| moved.get(&id).copied().unwrap_or(id));
    }
    for (id, mut group) in std::mem::take(&mut checker.group_inputs) {
        let id = moved[&id];
        group.input = moved.get(&group.input).copied().unwrap_or(group.input);
        checker.region_edges.insert(
            id,
            [
                Edge::new(Port::Entry(id), Port::Entry(group.input), Route::Next),
                Edge::new(Port::Normal(group.input), Port::Normal(id), Route::Next),
            ],
        );
        checker.group_inputs.insert(id, group);
    }
    assert_eq!(
        checker
            .grouped_consumer_limited(&reports, moved[&input], owner, Span::default(), depth)
            .unwrap(),
        Some(anchor)
    );
    let group = checker.group_inputs[&moved[&input]];
    for key in 0..=MAX_GROUPS {
        checker.group_inputs.insert(usize::MAX - key, group);
    }
    let error = checker
        .grouped_consumer_limited(&reports, anchor, owner, Span::default(), 0)
        .unwrap_err();
    assert!(error.message.contains("budget"));
}

#[test]
pub(crate) fn grouped_consumers_charge_exact_shared_work_and_combined_map_room() {
    let (mut checker, reports) = checked("a:(({->n:1})).n;b:-(({->2;->tag:true}))");
    let field = checker.fields.values().next().unwrap();
    let (input, owner) = (field.input, field.owner);
    let before = checker.flow.work;
    let anchor = checker
        .grouped_consumer_limited(&reports, input, owner, Span::default(), 2)
        .unwrap();
    let work = checker.flow.work - before;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.grouped_consumer_limited(&reports, input, owner, Span::default(), 2);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(result) = result {
            assert_eq!(result, anchor);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
    }
    checker.flow = crate::flow::Flow::new();
    let base = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len();
    let limit = base + reports.slot_uses.len();
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        reports.slot_uses
    );
    let work = checker.flow.work;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.slot_uses_limited(&reports, Span::default(), limit);
        assert_eq!(result.is_ok(), short == 0);
    }
    checker.flow = crate::flow::Flow::new();
    for room in [limit - 1, base - 1] {
        assert!(
            checker
                .slot_uses_limited(&reports, Span::default(), room)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
}

#[test]
pub(crate) fn grouped_consumers_deduplicate_extraction_visits_without_payload() {
    let (mut checker, mut reports) =
        checked("a:(({->n:1})).n;b:-(({->2;->tag:true}));c:(({->3;->tag:false}))+1");
    let effects = reports.effects.clone();
    let blocks = reports.blocks.clone();
    let results = reports.results.clone();
    let consumers = reports.consumers.clone();
    let uses = reports.slot_uses.clone();
    let parts = reports.parts;
    assert_eq!(uses.len(), 3);
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    assert_eq!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap(),
        effects
    );
    for _ in 0..3 {
        assert_eq!(checker.slot_uses(&reports, Span::default()).unwrap(), uses);
    }
    assert_eq!(reports.effects, effects);
    assert_eq!(reports.blocks, blocks);
    assert_eq!(reports.results, results);
    assert_eq!(reports.consumers, consumers);
    assert_eq!(reports.slot_uses, uses);
    assert_eq!(reports.parts, parts);
}

#[test]
pub(crate) fn grouped_consumers_discard_late_invalid_chains_atomically() {
    for fault in 0..3 {
        let (mut checker, reports) = checked("a:(({->n:1})).n;b:(({->n:2})).n");
        assert_eq!(reports.slot_uses.len(), 2);
        let mut fields = checker.fields.values();
        let first = fields.next().unwrap().input;
        let second = fields.next().unwrap().input;
        let group = checker.group_inputs[&second];
        let anchor = checker
            .grouped_consumer(&reports, first, group.owner, Span::default())
            .unwrap();
        assert!(anchor.is_some());
        match fault {
            0 => {
                checker.region_edges.remove(&second);
            }
            1 => checker.points[group.input].block = None,
            2 => checker.group_inputs.get_mut(&second).unwrap().owner += 1,
            _ => unreachable!(),
        }
        super::super::limits::rejected(&mut checker, &reports);
        assert_eq!(
            checker
                .grouped_consumer(&reports, first, group.owner, Span::default())
                .unwrap(),
            anchor
        );
    }
}
