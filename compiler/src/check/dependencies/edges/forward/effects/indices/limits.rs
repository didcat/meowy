use super::{super::tests::checked, *};

#[test]
pub(crate) fn index_effects_reject_corrupt_metadata_without_partial_reports() {
    for source in ["xs:[1];xs[1]", "xs:[1];p:&xs;p[1]"] {
        for fault in 0..34 {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, op) = checker.indices.first_key_value().unwrap();
            let receiver = op.receiver;
            let position = op.access.unwrap().position;
            let slot = 2 + usize::from(op.load);
            let before = reports.effects.clone();
            let op = checker.indices.get_mut(&id).unwrap();
            match fault {
                0 => op.owner = 9,
                1 => checker.points[id].owner = 9,
                2 => checker.points[id].kind = PointKind::Stmt,
                3 => checker.points[id].complete = false,
                4 => checker.points[id].span = Span::default(),
                5 => op.span = Span::default(),
                6 => op.access = None,
                7 => op.receiver = usize::MAX,
                8 => op.access.as_mut().unwrap().position = usize::MAX,
                9 => op.access.as_mut().unwrap().position = receiver,
                10 => op.access.as_mut().unwrap().capacity = crate::list::MAX_CAPACITY + 1,
                11 => op.access.as_mut().unwrap().length = Some(2),
                12 => op.access.as_mut().unwrap().may_return = false,
                13 => op.access.as_mut().unwrap().normal = false,
                14 => op.load = !op.load,
                15 => checker.points[receiver].parent = None,
                16 => checker.points[receiver].owner = 9,
                17 => checker.points[receiver].block = None,
                18 => checker.points[receiver].complete = false,
                19 => checker.points[receiver].kind = PointKind::Stmt,
                20 => checker.points[position].parent = None,
                21 => checker.points[position].owner = 9,
                22 => checker.points[position].block = None,
                23 => checker.points[position].complete = false,
                24 => checker.points[position].kind = PointKind::Stmt,
                25 => op.edges.clear(),
                26 => op.edges[0].from = Port::Entry(receiver),
                27 => op.edges[slot].to = Port::Entry(receiver),
                28 => op.edges[slot + 1].route = Route::Next,
                29 => op.edges[1].to = Port::Projection { point: id, step: 1 },
                30 => op.edges[slot + 2].to = Port::Normal(receiver),
                31 => {
                    reports.index.operations.remove(&id);
                }
                32 => {
                    reports.index.operations.insert(id, 9);
                }
                33 => checker.points.truncate(id),
                _ => unreachable!(),
            }
            let counts = checker.edge_counts();
            let indices = checker.indices.clone();
            assert_eq!(
                checker
                    .operation_effects(&reports, Span::default())
                    .unwrap_err()
                    .code,
                "B001",
                "{source}, fault {fault}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.indices, indices);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn index_effects_bound_total_work_with_duplicate_partial_stages() {
    let (mut checker, mut reports) = checked("xs:[1];p:&xs;xs[1];'out{p[{'out.leave()}]}", false);
    let ports = reports.entries[&0].1.ports.clone();
    reports.entries.get_mut(&0).unwrap().1.ports.extend(ports);
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
}

#[test]
pub(crate) fn index_effects_share_capacity_without_variable_payload_copies() {
    for spare in [0, 1] {
        let (mut checker, reports) = checked("xs:[1];p:&xs;xs[1];'out{p[{'out.leave()}]}", false);
        let expected = reports.effects.clone();
        let counts = checker.edge_counts();
        let result = checker.operation_effects_limited(
            &reports,
            Span::default(),
            expected.len() - spare,
            4,
            0,
        );
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(effects) = result {
            assert_eq!(effects, expected);
            assert_eq!(
                effects
                    .values()
                    .filter(|(_, effect)| matches!(effect, Effect::Index(_)))
                    .count(),
                2
            );
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn index_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, reports) = checked("xs:[1];p:&xs;p[1]", false);
    let id = *checker.indices.first_key_value().unwrap().0;
    let load = checker
        .index_effect_stage(
            &reports,
            0,
            Port::Projection { point: id, step: 0 },
            Span::default(),
        )
        .unwrap()
        .unwrap();
    let read = checker
        .index_effect_stage(&reports, 0, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_index_effect(load, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_index_effect(load, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..10 {
        let mut effects = expected.clone();
        let mut stage = read;
        match fault {
            0 => stage.owner += 1,
            1 => stage.receiver = usize::MAX,
            2 => stage.access.position = usize::MAX,
            3 => stage.access.capacity += 1,
            4 => stage.access.length = Some(0),
            5 => stage.access.may_return = false,
            6 => stage.access.normal = false,
            7 => stage.load = false,
            8 => stage.control = true,
            9 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_index_effect(stage, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_index_effect(read, &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}
