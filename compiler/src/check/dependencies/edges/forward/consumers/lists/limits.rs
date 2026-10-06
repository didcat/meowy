use super::super::{limits::rejected, tests::checked, *};
use crate::check::dependencies::{CoercionKind, SequenceSource};

pub(super) const SOURCE: &str = "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:{->7;->tag:true};|v<boolean>|xs<T[5]><U[5]>:[r,1,r,r,v]}";

#[test]
pub(crate) fn list_slots_keep_sparse_indices_and_independent_stage_observations() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let prior = reports.slot_uses.clone();
    assert_eq!(prior.len(), 3);
    for step in [0, 2, 3] {
        let (found, slot) = prior[&Port::Projection { point: id, step }];
        assert_eq!(found, owner);
        assert_ne!(owner, 0);
        assert_eq!(slot.index, 0);
        assert_eq!(checker.bodies[&slot.block].owner, owner);
    }
    let full = reports.effects[&id].1.clone();
    let port = Port::Projection { point: id, step: 3 };
    for state in 0..4 {
        let Effect::List(mut op) = full.clone() else {
            panic!()
        };
        op.constructed = state == 2;
        op.result = state == 3;
        for (part, input) in op.inputs.iter_mut().enumerate() {
            input.projected = state == 0 && part == 3;
            input.converted = state == 1 && part == 3;
        }
        reports.effects.get_mut(&id).unwrap().1 = Effect::List(op);
        let expected = if state == 0 {
            Uses::from([(port, prior[&port])])
        } else {
            Uses::new()
        };
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected
        );
        reports.index.operations.remove(&id);
        rejected(&mut checker, &reports);
        reports.index.operations.insert(id, owner);
    }
    assert_eq!(reports.slot_uses, prior);
}

#[test]
pub(crate) fn list_slots_keep_projected_stops_and_validate_unvisited_suffixes() {
    let (mut checker, mut reports) = checked(SOURCE);
    let id = *checker.lists.first_key_value().unwrap().0;
    let key = SequenceSource::Expr(id);
    let stop = Port::Projection { point: id, step: 2 };
    let prior = reports.slot_uses.clone();
    checker.list_inputs.get_mut(&id).unwrap()[2].kind = CoercionKind::Stopped;
    checker.list_inputs.get_mut(&id).unwrap()[2].source = Some(Shape::Never);
    let edges = &mut checker.sequences.get_mut(&key).unwrap().edges;
    checker.sequence_edges -= edges.len() - 2;
    edges.truncate(2);
    let ends = checker.endpoints.get_mut(&key).unwrap();
    let end = ends.iter().position(|edge| edge.to == stop).unwrap() + 1;
    checker.endpoint_edges -= ends.len() - end;
    ends.truncate(end);
    let Effect::List(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
        panic!()
    };
    op.inputs[2].plan = Some((true, CoercionKind::Stopped));
    op.inputs[2].source = Some(Shape::Never);
    for (part, input) in op.inputs.iter_mut().enumerate() {
        input.projected &= part <= 2;
        input.converted &= part < 2;
    }
    op.constructed = false;
    op.result = false;
    reports.index.operations.remove(&id);
    let expected: Uses = prior
        .iter()
        .filter(|(port, _)| **port != Port::Projection { point: id, step: 3 })
        .map(|(&port, &value)| (port, value))
        .collect();
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    let Effect::List(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
        panic!()
    };
    assert!(!op.inputs[4].projected && !op.inputs[4].converted);
    op.inputs[4].point = usize::MAX;
    rejected(&mut checker, &reports);
    let (_, reports) = checked(
        "<R>:<{-><never>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[2]><string[2]>:[v,{x:1;->x}]}",
    );
    assert!(reports.slot_uses.is_empty());
}

#[test]
pub(crate) fn list_slots_discard_late_plan_edge_and_source_failures() {
    for fault in 0..13 {
        let (mut checker, mut reports) = checked(&format!("a:{{->n:1}}.n;{SOURCE}"));
        let id = *checker.lists.first_key_value().unwrap().0;
        let key = SequenceSource::Expr(id);
        let input = checker.list_inputs[&id][4].point;
        let slot = reports.slot_uses[&Port::Projection { point: id, step: 3 }].1;
        assert!(checker.fields.keys().all(|field| *field < id));
        let Effect::List(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
            panic!()
        };
        match fault {
            0 => reports.effects.get_mut(&id).unwrap().0 += 1,
            1 => op.inputs[4].point = usize::MAX,
            2 => op.inputs[4].plan = None,
            3 => op.inputs.swap(0, 4),
            4 => checker.list_inputs.get_mut(&id).unwrap()[4].point = id,
            5 => checker.points[input].parent = None,
            6 => {
                checker
                    .endpoints
                    .get_mut(&key)
                    .unwrap()
                    .last_mut()
                    .unwrap()
                    .route = Route::Returned
            }
            7 => {
                checker
                    .sequences
                    .get_mut(&key)
                    .unwrap()
                    .edges
                    .last_mut()
                    .unwrap()
                    .route = Route::Checked
            }
            8 => {
                checker.lists.remove(&id);
            }
            9 => {
                reports.index.operations.insert(id, usize::MAX);
            }
            10 => reports.results.get_mut(&slot.block).unwrap().1.slots = None,
            11 => reports.blocks.get_mut(&slot.block).unwrap().1.result = false,
            12 => {
                op.inputs[1].projected = true;
            }
            _ => unreachable!(),
        }
        let eligible = reports.eligible.clone();
        let initializers = reports.initializers.clone();
        let lists = checker.lists.clone();
        let inputs = checker.list_inputs.clone();
        reports.parts = 0;
        rejected(&mut checker, &reports);
        assert_eq!(reports.eligible, eligible);
        assert_eq!(reports.initializers, initializers);
        assert_eq!(checker.lists, lists);
        assert_eq!(checker.list_inputs, inputs);
    }
}

#[test]
pub(crate) fn list_slots_deduplicate_visits_and_share_exact_map_and_work_limits() {
    let (mut checker, mut reports) = checked(&format!("a:{{->n:1}}.n;{SOURCE}"));
    let prior = reports.slot_uses.clone();
    let effects = reports.effects.clone();
    let results = reports.results.clone();
    let initializers = reports.initializers.clone();
    let lists = checker.lists.clone();
    let inputs = checker.list_inputs.clone();
    let counts = checker.edge_counts();
    assert_eq!(prior.len(), 4);
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
    let limit = base + prior.len();
    reports.parts = 0;
    let before = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        prior
    );
    let work = checker.flow.work - before;
    for room in [limit - 1, base - 1] {
        assert!(
            checker
                .slot_uses_limited(&reports, Span::default(), room)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.slot_uses_limited(&reports, Span::default(), limit);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(actual) = result {
            assert_eq!(actual, prior);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
    }
    assert_eq!(reports.slot_uses, prior);
    assert_eq!(reports.effects, effects);
    assert_eq!(reports.results, results);
    assert_eq!(reports.initializers, initializers);
    assert_eq!(reports.parts, 0);
    assert_eq!(checker.lists, lists);
    assert_eq!(checker.list_inputs, inputs);
    assert_eq!(checker.edge_counts(), counts);
}
