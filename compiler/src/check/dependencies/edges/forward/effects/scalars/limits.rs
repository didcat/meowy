use super::{super::tests::checked, *};

#[test]
pub(crate) fn scalar_effects_reject_corrupt_identity_and_edges_before_publication() {
    for normal in [false, true] {
        for fault in 0..18 {
            let (mut checker, mut reports) = checked("7", false);
            let id = *checker.scalar_leaves.first_key_value().unwrap().0;
            reports.entries.get_mut(&0).unwrap().1.ports = vec![if normal {
                Port::Normal(id)
            } else {
                Port::Operation(id)
            }];
            let before = reports.effects.clone();
            let leaf = checker.scalar_leaves.get_mut(&id).unwrap();
            match fault {
                0 => leaf.owner = 9,
                1 => checker.points[id].owner = 9,
                2 => checker.points[id].complete = false,
                3 => checker.points[id].kind = PointKind::Stmt,
                4 => checker.points[id].span = Span::default(),
                5 => leaf.span = Span::default(),
                6 => checker.points.truncate(id),
                7 => leaf.edges[0].from = Port::Entry(usize::MAX),
                8 => leaf.edges[0].to = Port::Normal(id),
                9 => leaf.edges[0].route = Route::Checked,
                10 => leaf.edges[1].from = Port::Entry(id),
                11 => leaf.edges[1].to = Port::Normal(usize::MAX),
                12 => leaf.edges[1].route = Route::Returned,
                13 => leaf.edges.swap(0, 1),
                14 => {
                    reports.index.operations.remove(&id);
                }
                15 => {
                    reports.index.operations.insert(id, 9);
                }
                16 => {
                    leaf.kind = ScalarKind::Int {
                        bits: 128,
                        signed: false,
                    }
                }
                17 => leaf.kind = ScalarKind::Float { bits: 16 },
                _ => unreachable!(),
            }
            let leaves = checker.scalar_leaves.clone();
            let counts = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(error.code, "B001", "fault {fault}, result {normal}");
            assert!(error.message.contains("scalar-effect identity"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.scalar_leaves, leaves);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn scalar_effects_bound_duplicate_work_and_shared_map_capacity() {
    let source = "a:7;b:\"text\";c:true";
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
        }
        assert_eq!(reports.effects, expected);
    }
}

#[test]
pub(crate) fn scalar_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, reports) = checked("7", false);
    let id = *checker.scalar_leaves.first_key_value().unwrap().0;
    let operation = checker
        .scalar_effect_stage(&reports, 0, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    let result = checker
        .scalar_effect_stage(&reports, 0, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_scalar_effect(operation, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_scalar_effect(operation, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..5 {
        let mut effects = expected.clone();
        let mut stage = result;
        match fault {
            0 => stage.owner += 1,
            1 => stage.ty = ScalarKind::Bool,
            2 => {
                stage.ty = ScalarKind::Int {
                    bits: 16,
                    signed: true,
                }
            }
            3 => stage.control = true,
            4 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_scalar_effect(stage, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_scalar_effect(result, &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}

#[test]
pub(crate) fn scalar_effect_work_does_not_scale_with_literal_payloads() {
    let mut costs = Vec::new();
    for len in [1, 65536] {
        let (mut checker, reports) = checked(&format!("\"{}\"", "x".repeat(len)), false);
        let id = *checker.scalar_leaves.first_key_value().unwrap().0;
        let before = checker.flow.work;
        let stage = checker
            .scalar_effect_stage(&reports, 0, Port::Normal(id), Span::default())
            .unwrap()
            .unwrap();
        assert_eq!(stage.ty, ScalarKind::String);
        costs.push(checker.flow.work - before);
        let expected = reports.effects.clone();
        assert_eq!(
            checker
                .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
                .unwrap(),
            expected
        );
    }
    assert_eq!(costs[0], costs[1]);
}

#[test]
pub(crate) fn scalar_effects_preserve_literal_type_and_stopped_tail_errors() {
    for (source, code) in [
        ("x<int8>:128", "E216"),
        ("x<int8>:-(128)", "E216"),
        ("x<uint8>:-1", "E222"),
        ("x<boolean>:1", "E207"),
        ("x:1e999", "E216"),
        ("d:@\"debug\";d.panic(\"stop\");x<int8>:128", "E216"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
