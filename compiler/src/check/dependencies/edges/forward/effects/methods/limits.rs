use super::{super::tests::checked, *};

#[test]
pub(crate) fn method_effects_reject_corrupt_add_metadata_without_partial_reports() {
    for source in [
        "xs<int32[2]>:[1];xs.add(2)",
        "xs<int32[2]>:[1];p:&xs;p.add(2)",
    ] {
        for fault in 0..34 {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, op) = checker.methods.first_key_value().unwrap();
            let receiver = op.receiver;
            let MethodKind::Add { item: input, .. } = op.kind else {
                panic!()
            };
            let slot = 2 + usize::from(op.load);
            let before = reports.effects.clone();
            let op = checker.methods.get_mut(&id).unwrap();
            let MethodKind::Add {
                item,
                capacity,
                length,
                may_return,
            } = &mut op.kind
            else {
                panic!()
            };
            match fault {
                0 => op.owner = 9,
                1 => checker.points[id].owner = 9,
                2 => checker.points[id].kind = PointKind::Stmt,
                3 => checker.points[id].complete = false,
                4 => checker.points[id].span = Span::default(),
                5 => op.span = Span::default(),
                6 => op.kind = MethodKind::Stopped,
                7 => op.receiver = usize::MAX,
                8 => *item = usize::MAX,
                9 => *item = receiver,
                10 => *capacity = crate::list::MAX_CAPACITY + 1,
                11 => *length = Some(3),
                12 => *may_return = false,
                13 => op.kind = MethodKind::StringSize,
                14 => op.load = !op.load,
                15 => checker.points[receiver].parent = None,
                16 => checker.points[receiver].owner = 9,
                17 => checker.points[receiver].block = None,
                18 => checker.points[receiver].complete = false,
                19 => checker.points[receiver].kind = PointKind::Stmt,
                20 => checker.points[input].parent = None,
                21 => checker.points[input].owner = 9,
                22 => checker.points[input].block = None,
                23 => checker.points[input].complete = false,
                24 => checker.points[input].kind = PointKind::Stmt,
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
            let methods = checker.methods.clone();
            assert_eq!(
                checker
                    .operation_effects(&reports, Span::default())
                    .unwrap_err()
                    .code,
                "B001",
                "{source}, fault {fault}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.methods, methods);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn method_effects_validate_size_edges_and_preserve_original_errors() {
    for source in ["xs:[1];xs.size()", "xs:[1];p:&xs;p.size()", "\"é\".size()"] {
        for fault in 0..6 {
            let (mut checker, reports) = checked(source, false);
            let id = *checker.methods.first_key_value().unwrap().0;
            let before = reports.effects.clone();
            let op = checker.methods.get_mut(&id).unwrap();
            let last = op.edges.len() - 1;
            match fault {
                0 => op.kind = MethodKind::Stopped,
                1 => op.load = !op.load,
                2 => op.edges[last].route = Route::Checked,
                3 => op.edges[last - 1].route = Route::Checked,
                4 => {
                    op.kind = MethodKind::Add {
                        item: op.receiver,
                        capacity: 2,
                        length: None,
                        may_return: true,
                    }
                }
                5 => op.edges[0].to = Port::Operation(id),
                _ => unreachable!(),
            }
            assert!(
                checker
                    .operation_effects(&reports, Span::default())
                    .is_err()
            );
            assert_eq!(reports.effects, before);
        }
    }
    for (source, code) in [
        ("xs:[1];xs.size(missing)", "E212"),
        ("xs:[1];xs.add(false)", "E207"),
        ("xs:[1];xs.add(2)", "E103"),
        ("xs:[1];xs.add()", "E212"),
        ("xs:1;xs.size()", "E201"),
        ("xs:=[1];p:&!(xs[1]);xs.size();after:*p", "E302"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn method_effects_bound_duplicate_stage_work_and_shared_effect_capacity() {
    let source = "xs<int32[2]>:[1];xs.size();p:&xs;'out{p.add({'out.leave()})}";
    let (mut checker, mut reports) = checked(source, false);
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
            assert_eq!(
                effects
                    .values()
                    .filter(|(_, effect)| matches!(effect, Effect::Method(_)))
                    .count(),
                2
            );
        }
        assert_eq!(reports.effects, expected);
    }
}

#[test]
pub(crate) fn method_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, reports) = checked("xs<int32[2]>:[1];p:&xs;p.add(2)", false);
    let id = *checker.methods.first_key_value().unwrap().0;
    let load = checker
        .method_effect_stage(
            &reports,
            0,
            Port::Projection { point: id, step: 0 },
            Span::default(),
        )
        .unwrap()
        .unwrap();
    let finish = checker
        .method_effect_stage(&reports, 0, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_method_effect(load, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_method_effect(load, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..10 {
        let mut effects = expected.clone();
        let mut stage = finish;
        let MethodKind::Add {
            item,
            capacity,
            length,
            may_return,
        } = &mut stage.method
        else {
            panic!()
        };
        match fault {
            0 => stage.owner += 1,
            1 => stage.receiver = usize::MAX,
            2 => stage.method = MethodKind::ListSize,
            3 => *item = usize::MAX,
            4 => *capacity += 1,
            5 => *length = Some(0),
            6 => *may_return = false,
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
                .record_method_effect(stage, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_method_effect(finish, &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}
