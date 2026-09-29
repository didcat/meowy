use super::{super::tests::checked, *};

#[test]
pub(crate) fn unary_effects_reject_corrupt_roots_and_checked_edges_atomically() {
    for source in ["x:!false", "x:-({->2;->tag:true})"] {
        for fault in 0..25 {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, op) = checker.unaries.first_key_value().unwrap();
            let input = op.input;
            let slot = op.edges.len() - 2;
            let before = reports.effects.clone();
            let op = checker.unaries.get_mut(&id).unwrap();
            match fault {
                0 => op.owner = 9,
                1 => checker.points[id].owner = 9,
                2 => checker.points[id].complete = false,
                3 => checker.points[id].kind = PointKind::Stmt,
                4 => checker.points[id].span = Span::default(),
                5 => op.span = Span::default(),
                6 => op.input = usize::MAX,
                7 => checker.points[input].parent = None,
                8 => checker.points[input].owner = 9,
                9 => checker.points[input].block = None,
                10 => checker.points[input].complete = false,
                11 => checker.points[input].kind = PointKind::Stmt,
                12 => op.kind = UnaryKind::Stopped,
                13 => op.primary = !op.primary,
                14 => op.edges.clear(),
                15 => op.edges[0].to = Port::Normal(input),
                16 => op.edges[slot].from = Port::Entry(input),
                17 => op.edges[slot + 1].to = Port::Normal(input),
                18 => {
                    op.edges[slot + 1].route = if op.edges[slot + 1].route == Route::Checked {
                        Route::Next
                    } else {
                        Route::Checked
                    }
                }
                19 => op.edges[slot].to = Port::Normal(id),
                20 => op.edges[slot].route = Route::Returned,
                21 => {
                    reports.index.operations.remove(&id);
                }
                22 => {
                    reports.index.operations.insert(id, 9);
                }
                23 => checker.points.truncate(id),
                24 => {
                    op.kind = if op.kind == UnaryKind::Not {
                        UnaryKind::Negate
                    } else {
                        UnaryKind::BitsNot
                    }
                }
                _ => unreachable!(),
            }
            let counts = checker.edge_counts();
            let unaries = checker.unaries.clone();
            assert_eq!(
                checker
                    .operation_effects(&reports, Span::default())
                    .unwrap_err()
                    .code,
                "B001",
                "{source}, fault {fault}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.unaries, unaries);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn unary_effects_reject_invalid_types_before_any_shape_copy_or_walk() {
    let mut types = vec![
        Type::Null,
        Type::Never,
        Type::Bool,
        Type::String,
        Type::Int {
            bits: 0,
            signed: true,
        },
        Type::Int {
            bits: 128,
            signed: true,
        },
        Type::Int {
            bits: 8,
            signed: false,
        },
        Type::Float { bits: 16 },
        Type::Reference(Box::new(Type::Bool)),
        Type::List {
            element: Box::new(Type::Bool),
            capacity: 1,
        },
        Type::Union(vec![Type::Bool, Type::Null]),
    ];
    for size in [0, 1024] {
        types.push(Type::Record {
            primary: Box::new(Type::Null),
            fields: (0..size)
                .map(|index| crate::hir::Field {
                    name: index.to_string(),
                    ty: Type::Bool,
                    mutable: false,
                })
                .collect(),
        });
    }
    let mut work = None;
    for ty in types {
        let (mut checker, reports) = checked("n:=7;x:-n", false);
        let id = *checker.unaries.first_key_value().unwrap().0;
        let effects = reports.effects.clone();
        checker.unaries.get_mut(&id).unwrap().ty = ty;
        let before = checker.flow.work;
        let error = checker
            .unary_effect_stage(&reports, 0, Port::Operation(id), Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity"));
        let spent = checker.flow.work - before;
        if let Some(expected) = work {
            assert_eq!(spent, expected);
        } else {
            work = Some(spent);
        }
        assert_eq!(reports.effects, effects);
    }
    for (source, code) in [
        ("x:!1", "E222"),
        ("n<uint8>:7;x:-n", "E222"),
        ("x<int8>:-(128)", "E216"),
        ("n<int8>:-128;x:-n", "E107"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn unary_effects_bound_duplicate_stage_work_and_fixed_payload_capacity() {
    let source = "a:!false;x:-({->2;->tag:true})";
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
pub(crate) fn unary_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, reports) = checked("x:-({->2;->tag:true})", false);
    let id = *checker.unaries.first_key_value().unwrap().0;
    let projection = checker
        .unary_effect_stage(
            &reports,
            0,
            Port::Projection { point: id, step: 0 },
            Span::default(),
        )
        .unwrap()
        .unwrap();
    let result = checker
        .unary_effect_stage(&reports, 0, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_unary_effect(projection, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_unary_effect(projection, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..9 {
        let mut effects = expected.clone();
        let mut stage = result;
        match fault {
            0 => stage.owner += 1,
            1 => stage.input = usize::MAX,
            2 => stage.op = UnaryKind::BitsNot,
            3 => stage.ty = ScalarKind::Bool,
            4 => {
                stage.ty = ScalarKind::Int {
                    bits: 16,
                    signed: true,
                }
            }
            5 => stage.primary = false,
            6 => stage.checked = false,
            7 => stage.control = true,
            8 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_unary_effect(stage, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_unary_effect(result, &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}
