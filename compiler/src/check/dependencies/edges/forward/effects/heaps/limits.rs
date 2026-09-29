use super::{super::tests::checked, *};

#[test]
pub(crate) fn heap_effects_preserve_nominal_borrow_and_lifetime_diagnostics() {
    for (tail, code) in [
        ("a<m.Allocator>:{->n:1}", "E207"),
        ("p:&{->m.heap};copy:*p", "E303"),
        ("a:=m.heap;p:&a;a=m.heap;copy:*p", "E302"),
        ("a:m.heap;same:a==a", "E222"),
        ("copy:*(&m.heap)", "B001"),
        ("m.missing", "B001"),
        ("d.print(m.heap)", "B001"),
        ("d.panic(\"stop\");a:m.heap;same:a==a", "E222"),
    ] {
        let source = format!("m:@\"memory\";d:@\"debug\";{tail}");
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, code, "{tail}");
    }
}

#[test]
pub(crate) fn heap_effects_reject_corrupt_identity_and_edges_before_publication() {
    for normal in [false, true] {
        for fault in 0..18 {
            let (mut checker, mut reports) = checked("m:@\"memory\";m.heap", false);
            let id = *checker.heap_leaves.first_key_value().unwrap().0;
            reports.entries.get_mut(&0).unwrap().1.ports = vec![if normal {
                Port::Normal(id)
            } else {
                Port::Operation(id)
            }];
            let before = reports.effects.clone();
            let leaf = checker.heap_leaves.get_mut(&id).unwrap();
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
                16 => leaf.ty = FoundationType::AllocationFailure,
                17 => leaf.ty = FoundationType::OwnedString,
                _ => unreachable!(),
            }
            let leaves = checker.heap_leaves.clone();
            let counts = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(error.code, "B001", "fault {fault}, result {normal}");
            assert!(error.message.contains("heap-effect identity"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.heap_leaves, leaves);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn heap_effects_bound_duplicate_work_and_shared_map_capacity() {
    let source = "m:@\"memory\";a:m.heap;b:m.heap;c:7";
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
pub(crate) fn heap_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, reports) = checked("m:@\"memory\";m.heap", false);
    let id = *checker.heap_leaves.first_key_value().unwrap().0;
    let operation = checker
        .heap_effect_stage(&reports, 0, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    let result = checker
        .heap_effect_stage(&reports, 0, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_heap_effect(operation, &mut effects, 0, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    checker
        .record_heap_effect(operation, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for fault in 0..4 {
        let mut effects = expected.clone();
        let mut stage = result;
        match fault {
            0 => stage.owner += 1,
            1 => stage.ty = FoundationType::AllocationFailure,
            2 => stage.control = true,
            3 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_heap_effect(stage, &mut effects, 1, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
    }
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_heap_effect(result, &mut effects, 1, Span::default())
            .is_err()
    );
    assert_eq!(effects, expected);
}
