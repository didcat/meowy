use super::{
    super::{limits::rejected, tests::checked},
    *,
};
use crate::check::dependencies::CoercionKind;

#[test]
pub(crate) fn coercion_slots_keep_seeded_stopped_projection_at_a_real_record_anchor() {
    let (mut checker, mut reports) = checked("r:{->7;->tag:true};x<int32>:r");
    let (&id, _) = checker.coercions.iter().find(|(_, op)| op.primary).unwrap();
    let port = Port::Projection { point: id, step: 0 };
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 1);
    let (owner, slot) = expected[&port];
    let anchor = reports.results[&slot.block].1.consumer.unwrap();
    assert_eq!(reports.consumers[&anchor], (owner, slot.block));
    assert_eq!(slot.index, 0);
    let op = checker.coercions.get_mut(&id).unwrap();
    assert_eq!(op.kind, CoercionKind::Forward);
    assert_eq!(op.edges.pop().unwrap().to, Port::Normal(id));
    op.kind = CoercionKind::Stopped;
    op.source = Some(Shape::Never);
    checker.coercion_edges -= 1;
    let Effect::Coercion(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
        panic!()
    };
    op.op = CoercionKind::Stopped;
    op.source = Some(Shape::Never);
    op.result = false;
    assert!(op.projected && !op.operation);
    assert!(!reports.index.operations.contains_key(&id));
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    assert_eq!(
        checker
            .grouped_consumer(&reports, id, owner, Span::default())
            .unwrap(),
        None
    );
    for result in [false, true] {
        let Effect::Coercion(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
            panic!()
        };
        op.operation = !result;
        op.result = result;
        rejected(&mut checker, &reports);
    }
}

#[test]
pub(crate) fn coercion_slots_require_conversion_registration_only_for_observed_later_stages() {
    let (mut checker, mut reports) = checked("r:{->7;->tag:true};x<int32><null>:r");
    let (&id, op) = checker.coercions.iter().find(|(_, op)| op.primary).unwrap();
    assert_eq!(op.kind, CoercionKind::Convert);
    let owner = op.owner;
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 1);
    let Effect::Coercion(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
        panic!()
    };
    op.operation = false;
    op.result = false;
    reports.index.operations.remove(&id);
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    for result in [false, true] {
        let Effect::Coercion(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
            panic!()
        };
        op.operation = !result;
        op.result = result;
        rejected(&mut checker, &reports);
    }
    reports.index.operations.insert(id, owner);
    for result in [false, true] {
        let Effect::Coercion(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
            panic!()
        };
        op.projected = false;
        op.operation = !result;
        op.result = result;
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap()
                .is_empty()
        );
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn coercion_slots_discard_late_header_stage_and_edge_failures_atomically() {
    for fault in 0..18 {
        let source = "a:{->n:1}.n;r:{->7;->tag:true};x<int32><null>:r";
        let (mut checker, mut reports) = checked(source);
        let (&id, op) = checker.coercions.iter().find(|(_, op)| op.primary).unwrap();
        let input = op.input;
        assert!(checker.fields.keys().all(|field| *field < id));
        let Effect::Coercion(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
            panic!()
        };
        match fault {
            0 => op.input = usize::MAX,
            1 => op.op = CoercionKind::Forward,
            2 => op.primary = false,
            3 => op.control = !op.control,
            4 => {
                op.projected = false;
                op.operation = false;
                op.result = false;
            }
            5 => reports.effects.get_mut(&id).unwrap().0 += 1,
            6 => checker.coercions.get_mut(&id).unwrap().owner += 1,
            7 => checker.points[id].complete = false,
            8 => checker.coercions.get_mut(&id).unwrap().input = id,
            9 => checker.points[input].parent = None,
            10 => checker.coercions.get_mut(&id).unwrap().edges[1].route = Route::Checked,
            11 => {
                let op = checker.coercions.get_mut(&id).unwrap();
                op.edges.push(op.edges[0]);
            }
            12 => {
                reports.index.operations.remove(&id);
            }
            13 => {
                reports.index.operations.insert(id, usize::MAX);
            }
            14 => {
                checker.coercions.remove(&id);
            }
            15 => checker.points[input].span = Span::default(),
            16 => checker.points[id].kind = PointKind::Stmt,
            17 => {
                let (_, slot) = reports.slot_uses[&Port::Projection { point: id, step: 0 }];
                reports.results.get_mut(&slot.block).unwrap().1.slots = None;
            }
            _ => unreachable!(),
        }
        let eligible = reports.eligible.clone();
        let initializers = reports.initializers.clone();
        let coercions = checker.coercions.clone();
        reports.parts = 0;
        rejected(&mut checker, &reports);
        assert_eq!(reports.eligible, eligible);
        assert_eq!(reports.initializers, initializers);
        assert_eq!(checker.coercions, coercions);
    }
    let (mut checker, mut reports) = checked("r:{->7;->tag:true};x<int32>:r");
    let op = reports
        .effects
        .values_mut()
        .find_map(|(_, effect)| match effect {
            Effect::Coercion(op) if op.primary => Some(op),
            _ => None,
        })
        .unwrap();
    op.operation = true;
    rejected(&mut checker, &reports);
}

#[test]
pub(crate) fn coercion_slots_deduplicate_visits_and_share_exact_capacity_and_work_limits() {
    let source = "a:{->n:1}.n;r:{->7;->tag:true};x<int32>:r;y<int32><null>:r";
    let (mut checker, mut reports) = checked(source);
    let expected = reports.slot_uses.clone();
    let effects = reports.effects.clone();
    let results = reports.results.clone();
    let initializers = reports.initializers.clone();
    let counts = checker.edge_counts();
    assert_eq!(expected.len(), 3);
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert_eq!(reports.effects, effects);
    let base = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len();
    let limit = base + expected.len();
    reports.parts = 0;
    let before = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        expected
    );
    let work = checker.flow.work - before;
    for room in [limit - 1, base - 1] {
        let error = checker
            .slot_uses_limited(&reports, Span::default(), room)
            .unwrap_err();
        assert!(error.message.contains("budget"));
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let actual = checker.slot_uses_limited(&reports, Span::default(), limit);
        assert_eq!(actual.is_ok(), short == 0);
        if let Ok(actual) = actual {
            assert_eq!(actual, expected);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
    }
    assert_eq!(reports.slot_uses, expected);
    assert_eq!(reports.effects, effects);
    assert_eq!(reports.results, results);
    assert_eq!(reports.initializers, initializers);
    assert_eq!(reports.parts, 0);
    assert_eq!(checker.edge_counts(), counts);
}
