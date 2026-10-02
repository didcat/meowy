use super::{super::tests::checked, *};

#[test]
pub(crate) fn coercion_effects_reject_corrupt_sources_and_edges_atomically() {
    for source in [
        "n:7;x<int32>:n",
        "n:7;x<int32><null>:n",
        "r:{->7;->tag:true};x<int32>:r",
        "r:{->7;->tag:true};x<int32><null>:r",
    ] {
        for fault in 0..30 {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, op) = checker
                .coercions
                .iter()
                .find(|(_, op)| op.primary)
                .or_else(|| checker.coercions.last_key_value())
                .unwrap();
            let (owner, input, kind) = (op.owner, op.input, op.kind);
            if matches!(fault, 25 | 26) && kind != CoercionKind::Convert {
                continue;
            }
            reports.entries.get_mut(&owner).unwrap().1.ports = vec![Port::Normal(id)];
            let before = reports.effects.clone();
            let op = checker.coercions.get_mut(&id).unwrap();
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
                13 => checker.points[input].span = Span::default(),
                14 => checker.points.truncate(id),
                15 => {
                    op.kind = if kind == CoercionKind::Convert {
                        CoercionKind::Forward
                    } else {
                        CoercionKind::Convert
                    }
                }
                16 => op.kind = CoercionKind::Stopped,
                17 => op.edges.clear(),
                18 => op.edges[0].from = Port::Entry(input),
                19 => op.edges[0].to = Port::Normal(input),
                20 => op.edges[1].from = Port::Entry(input),
                21 => op.edges[last].to = Port::Normal(input),
                22 => op.edges[last].route = Route::Checked,
                23 => op.edges.push(op.edges[last]),
                24 => op.edges.swap(0, 1),
                25 => {
                    reports.index.operations.remove(&id);
                }
                26 => {
                    reports.index.operations.insert(id, 9);
                }
                27 => op.primary = !op.primary,
                28 => {
                    reports.entries.get_mut(&owner).unwrap().1.ports =
                        vec![Port::Projection { point: id, step: 1 }]
                }
                29 => op.edges[1].to = Port::Projection { point: id, step: 1 },
                _ => unreachable!(),
            }
            let counts = checker.edge_counts();
            let ops = checker.coercions.clone();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(error.code, "B001", "{source}, fault {fault}");
            assert!(error.message.contains("coercion-effect identity"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.coercions, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn coercion_effects_bound_duplicate_work_and_shared_capacity() {
    let source = "n:7;a<int32>:n;r:{->7;->tag:true};x<int32><null>:r";
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
            7,
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
pub(crate) fn coercion_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, reports) = checked("r:{->7;->tag:true};x<int32><null>:r", false);
    let (&id, op) = checker
        .coercions
        .iter()
        .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
        .unwrap();
    let owner = op.owner;
    let projection = checker
        .coercion_effect_stage(
            &reports,
            owner,
            Port::Projection { point: id, step: 0 },
            Span::default(),
        )
        .unwrap()
        .unwrap();
    let result = checker
        .coercion_effect_stage(&reports, owner, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_coercion_effect(projection, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_coercion_effect(projection, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..6 {
        let mut effects = expected.clone();
        let mut stage = result;
        match fault {
            0 => stage.owner += 1,
            1 => stage.input = usize::MAX,
            2 => stage.op = CoercionKind::Forward,
            3 => stage.primary = false,
            4 => stage.control = true,
            5 => {
                effects.insert(id, (owner, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_coercion_effect(stage, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_coercion_effect(result, &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}
