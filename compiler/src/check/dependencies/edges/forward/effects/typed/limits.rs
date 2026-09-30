use super::{super::tests::checked, *};

#[test]
pub(crate) fn typed_effects_reject_corrupt_sources_and_edges_atomically() {
    for source in ["7<int32>", "7~<int32>"] {
        for (normal, fault) in [false, true]
            .into_iter()
            .flat_map(|normal| (0..25).map(move |fault| (normal, fault)))
        {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, op) = checker.typed_ops.first_key_value().unwrap();
            let (owner, input) = (op.owner, op.input);
            reports.entries.get_mut(&owner).unwrap().1.ports = vec![if normal {
                Port::Normal(id)
            } else {
                Port::Operation(id)
            }];
            let before = reports.effects.clone();
            let op = checker.typed_ops.get_mut(&id).unwrap();
            let last = op.edges.len() - 1;
            match fault {
                0 => op.owner = 9,
                1 => checker.points[id].owner = 9,
                2 => checker.points[id].complete = false,
                3 => checker.points[id].kind = PointKind::Stmt,
                4 => checker.points[id].span = Span::default(),
                5 => op.span = Span::default(),
                6 => op.input = usize::MAX,
                7 => op.input = id,
                8 => checker.points[input].parent = None,
                9 => checker.points[input].owner = 9,
                10 => checker.points[input].block = None,
                11 => checker.points[input].complete = false,
                12 => checker.points[input].kind = PointKind::Stmt,
                13 => checker.points.truncate(id),
                14 => op.normal = false,
                15 => op.edges.clear(),
                16 => op.edges[0].from = Port::Entry(input),
                17 => op.edges[0].to = Port::Normal(input),
                18 => op.edges[1].from = Port::Entry(input),
                19 => op.edges[last].to = Port::Normal(input),
                20 => op.edges[last].route = Route::Checked,
                21 => op.edges.push(op.edges[last]),
                22 => op.edges.swap(0, 1),
                23 => {
                    reports.index.operations.remove(&id);
                }
                24 => {
                    reports.index.operations.insert(id, 9);
                }
                _ => unreachable!(),
            }
            let counts = checker.edge_counts();
            let ops = checker.typed_ops.clone();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(
                error.code, "B001",
                "{source}, fault {fault}, result {normal}"
            );
            assert!(error.message.contains("typed-effect identity"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.typed_ops, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn typed_effects_bound_duplicate_work_and_shared_capacity() {
    let source = "v:7<int32>;x:(7~<int32>)<int32>";
    let (mut checker, mut reports) = checked(source, false);
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    let expected = reports.effects.clone();
    let counts = checker.edge_counts();
    let before = checker.flow.work;
    assert_eq!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap(),
        expected
    );
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.operation_effects(&reports, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(effects) = result {
            assert_eq!(effects, expected);
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
    }
    for spare in [0, 1] {
        let (mut checker, reports) = checked(source, false);
        let expected = reports.effects.clone();
        let result = checker.operation_effects_limited(
            &reports,
            Span::default(),
            expected.len() - spare,
            0,
            0,
        );
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(effects) = result {
            assert_eq!(effects, expected);
        }
        assert_eq!(reports.effects, expected);
    }
}

#[test]
pub(crate) fn typed_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, reports) = checked("7<int32>", false);
    let (&id, op) = checker.typed_ops.first_key_value().unwrap();
    let owner = op.owner;
    let operation = checker
        .typed_effect_stage(&reports, owner, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    let result = checker
        .typed_effect_stage(&reports, owner, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_typed_effect(operation, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_typed_effect(operation, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..6 {
        let mut effects = expected.clone();
        let mut stage = result;
        match fault {
            0 => stage.owner += 1,
            1 => stage.input = usize::MAX,
            2 => stage.op = TypedKind::Ascription,
            3 => stage.normal = false,
            4 => stage.control = true,
            5 => {
                effects.insert(id, (owner, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_typed_effect(stage, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_typed_effect(result, &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}
