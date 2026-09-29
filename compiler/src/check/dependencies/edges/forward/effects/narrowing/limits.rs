use super::{super::tests::checked, *};

#[test]
pub(crate) fn narrowing_effects_reject_corrupt_sources_and_edges_atomically() {
    for source in ["v:1;v", "f:(v<int32><null>){|v<int32>|v}"] {
        for fault in 0..27 {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, op) = checker
                .narrowings
                .iter()
                .find(|(_, op)| op.changed)
                .or_else(|| checker.narrowings.first_key_value())
                .unwrap();
            let (owner, input, changed) = (op.owner, op.input, op.changed);
            if fault >= 25 && !changed {
                continue;
            }
            reports.entries.get_mut(&owner).unwrap().1.ports = vec![Port::Normal(id)];
            let before = reports.effects.clone();
            let op = checker.narrowings.get_mut(&id).unwrap();
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
                15 => op.changed = !changed,
                16 => op.normal = false,
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
                _ => unreachable!(),
            }
            let counts = checker.edge_counts();
            let ops = checker.narrowings.clone();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(error.code, "B001", "{source}, fault {fault}");
            assert!(error.message.contains("narrowing-effect identity"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.narrowings, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn narrowing_effects_bound_duplicate_work_and_shared_capacity() {
    let source = "v:1;x:v;f:(v<int32><null>){|v<int32>|v}";
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
pub(crate) fn narrowing_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, reports) = checked("f:(v<int32><null>){|v<int32>|v}", false);
    let (&id, op) = checker
        .narrowings
        .iter()
        .find(|(_, op)| op.changed)
        .unwrap();
    let owner = op.owner;
    let operation = checker
        .narrowing_effect_stage(&reports, owner, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    let result = checker
        .narrowing_effect_stage(&reports, owner, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_narrowing_effect(operation, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_narrowing_effect(operation, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..6 {
        let mut effects = expected.clone();
        let mut stage = result;
        match fault {
            0 => stage.owner += 1,
            1 => stage.input = usize::MAX,
            2 => stage.changed = false,
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
                .record_narrowing_effect(stage, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_narrowing_effect(result, &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}

#[test]
pub(crate) fn narrowing_effects_keep_conversion_to_never_without_a_result() {
    let (mut checker, mut reports) = checked("v:true", false);
    let ty = crate::hir::Type::union(vec![crate::hir::Type::Bool, crate::hir::Type::Null]);
    checker.tags.insert(
        ((0, Vec::new()), ty.clone()),
        vec![
            (crate::hir::Type::Bool, crate::flow::FALSE),
            (crate::hir::Type::Null, crate::flow::FALSE),
        ],
    );
    let span = Span::new(1, 2);
    let (id, value) = checker
        .with_point_id(PointKind::Expr, span, |checker| {
            checker.narrow_source(span, |_| {
                Ok(crate::hir::Expr {
                    kind: crate::hir::ExprKind::Local(0),
                    ty,
                    span,
                })
            })
        })
        .unwrap();
    assert_eq!(value.ty, crate::hir::Type::Never);
    reports.index.operations.insert(id, 0);
    reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Operation(id)];
    let effects = checker.operation_effects(&reports, span).unwrap();
    let (_, Effect::Narrowing(op)) = &effects[&id] else {
        panic!()
    };
    assert!(op.changed && op.operation && !op.normal && !op.result);
    reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Normal(id)];
    assert!(checker.operation_effects(&reports, span).is_err());
    reports.index.operations.remove(&id);
    assert!(
        checker
            .narrowing_effect_stage(&reports, 0, Port::Operation(id), span)
            .is_err()
    );
}

#[test]
pub(crate) fn narrowing_effects_preserve_type_ascription_mutation_and_borrow_errors() {
    for (source, code) in [
        ("f:(v<int32><null>){x<int32>:v}", "E207"),
        ("f:(v<int32><null>){x:v~<int32>}", "E208"),
        ("v<int32><null>:=1;|v<int32>|{v=null;x<int32>:v}", "E207"),
        ("v:=1;p:&v;v=2;after:*p", "E302"),
        (
            "d:@\"debug\";v<int32><null>:=1;d.panic(\"stop\");x<int32>:v",
            "E207",
        ),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
