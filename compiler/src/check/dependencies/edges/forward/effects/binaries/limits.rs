use super::{super::tests::checked, *};

#[test]
pub(crate) fn binary_effects_reject_corrupt_roots_and_both_edge_ledgers_atomically() {
    for source in ["1+2", "a:{->1;->tag:true};b:{->2;->tag:false};a+b"] {
        for fault in 0..40 {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, op) = checker.binaries.first_key_value().unwrap();
            let [left, right] = op.inputs;
            let slot = op.edges.len() - 2;
            let key = SequenceSource::Expr(id);
            let before = reports.effects.clone();
            let op = checker.binaries.get_mut(&id).unwrap();
            match fault {
                0 => op.owner = 9,
                1 => checker.points[id].owner = 9,
                2 => checker.points[id].complete = false,
                3 => checker.points[id].kind = PointKind::Stmt,
                4 => checker.points[id].span = Span::default(),
                5 => op.span = Span::default(),
                6 => op.inputs[0] = usize::MAX,
                7 => op.inputs[1] = usize::MAX,
                8 => op.inputs[1] = left,
                9 => checker.points[left].parent = None,
                10 => checker.points[left].owner = 9,
                11 => checker.points[left].block = None,
                12 => checker.points[left].complete = false,
                13 => checker.points[left].kind = PointKind::Stmt,
                14 => checker.points[right].parent = None,
                15 => checker.points[right].owner = 9,
                16 => checker.points[right].block = None,
                17 => checker.points[right].complete = false,
                18 => checker.points[right].kind = PointKind::Stmt,
                19 => op.plan.primary[0] = !op.plan.primary[0],
                20 => op.plan.primary[1] = !op.plan.primary[1],
                21 => op.plan.normal[0] = false,
                22 => op.plan.normal[1] = false,
                23 => op.edges.clear(),
                24 => op.edges[0].to = Port::Normal(left),
                25 => op.edges[slot].from = Port::Entry(right),
                26 => op.edges[slot + 1].to = Port::Normal(left),
                27 => op.edges[slot + 1].route = Route::Next,
                28 => op.edges[slot].route = Route::Checked,
                29 => checker.sequences.get_mut(&key).unwrap().owner = 9,
                30 => checker.sequences.get_mut(&key).unwrap().items[0] = None,
                31 => checker.sequences.get_mut(&key).unwrap().items.swap(0, 1),
                32 => checker.sequences.get_mut(&key).unwrap().edges.clear(),
                33 => {
                    checker.sequences.remove(&key);
                }
                34 => {
                    checker.sequences.get_mut(&key).unwrap().edges[0].from = if op.plan.primary[0] {
                        Port::Normal(left)
                    } else {
                        Port::Projection { point: id, step: 0 }
                    }
                }
                35 => checker.sequences.get_mut(&key).unwrap().edges[0].to = Port::Entry(left),
                36 => checker.sequences.get_mut(&key).unwrap().edges[0].route = Route::Checked,
                37 => {
                    reports.index.operations.remove(&id);
                }
                38 => {
                    reports.index.operations.insert(id, 9);
                }
                39 => checker.points.truncate(id),
                _ => unreachable!(),
            }
            let counts = checker.edge_counts();
            let binaries = checker.binaries.clone();
            assert_eq!(
                checker
                    .operation_effects(&reports, Span::default())
                    .unwrap_err()
                    .code,
                "B001",
                "{source}, fault {fault}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.binaries, binaries);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn binary_effects_reject_invalid_signatures_and_preserve_diagnostics() {
    for fault in 0..11 {
        let (mut checker, reports) = checked("1+2", false);
        let id = *checker.binaries.first_key_value().unwrap().0;
        let before = reports.effects.clone();
        let op = checker.binaries.get_mut(&id).unwrap();
        match fault {
            0 => op.op = "&&".into(),
            1 => {
                op.types.inputs[0] = Class::Scalar(ScalarKind::Int {
                    bits: 0,
                    signed: true,
                })
            }
            2 => {
                op.types.inputs[1] = Class::Scalar(ScalarKind::Int {
                    bits: 16,
                    signed: true,
                })
            }
            3 => op.types.result = Class::Scalar(ScalarKind::Bool),
            4 => op.types.result = Class::Never,
            5 => op.types.inputs[0] = Class::Never,
            6 => op.plan.checked = false,
            7 => {
                op.types.inputs = [Class::Scalar(ScalarKind::Bool); 2];
                op.types.result = Class::Scalar(ScalarKind::Bool);
                op.plan.checked = false;
            }
            8 | 9 => {
                let ty = Class::Scalar(ScalarKind::Float {
                    bits: if fault == 8 { 32 } else { 16 },
                });
                op.types.inputs = [ty; 2];
                op.types.result = ty;
                op.op = "%".into();
                op.plan.checked = false;
            }
            10 => {
                op.types.inputs = [Class::Other; 2];
                op.types.result = Class::Other;
                op.plan.checked = false;
            }
            _ => unreachable!(),
        }
        assert!(
            checker
                .operation_effects(&reports, Span::default())
                .is_err()
        );
        assert_eq!(reports.effects, before);
    }
    for (source, code) in [
        ("1/0", "E107"),
        ("1+false", "E222"),
        ("a<int8>:1;b<int16>:2;a+b", "E213"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn binary_effects_bound_duplicate_stage_work_and_fixed_payload_capacity() {
    let source =
        "f<never>:(r<{-><never>;tag<boolean>}>){->r+1};a:{->1;->tag:true};b:{->2;->tag:false};a+b";
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
pub(crate) fn binary_effects_preserve_partial_records_on_conflicts_and_invalid_selectors() {
    let (mut checker, reports) = checked("a:{->1;->tag:true};b:{->2;->tag:false};a+b", false);
    let id = *checker.binaries.first_key_value().unwrap().0;
    let projection = checker
        .binary_effect_stage(
            &reports,
            0,
            Port::Projection { point: id, step: 0 },
            Span::default(),
        )
        .unwrap()
        .unwrap();
    let result = checker
        .binary_effect_stage(&reports, 0, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_binary_effect(projection, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_binary_effect(projection, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..11 {
        let mut effects = expected.clone();
        let mut stage = result;
        match fault {
            0 => stage.owner += 1,
            1 => stage.inputs[0] = usize::MAX,
            2 => stage.op = "-",
            3 => stage.types.inputs[0] = Class::Scalar(ScalarKind::Bool),
            4 => stage.types.result = Class::Never,
            5 => stage.plan.primary[0] = false,
            6 => stage.plan.normal[0] = false,
            7 => stage.plan.checked = false,
            8 => stage.control = true,
            9 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            10 => stage.kind = Kind::Projection(usize::MAX),
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_binary_effect(stage, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_binary_effect(result, &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}
