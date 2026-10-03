use super::{super::tests::checked, *};

#[test]
pub(crate) fn forward_coercion_inputs_retain_typed_initializer_roots() {
    let source = "flag:false;r<{n<int32>}>:(({->n:1}));|flag|x<int32>:7;f<int32>:(){->8}";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, true);
    let ids: Vec<_> = checker
        .coercions
        .iter()
        .filter(|(_, op)| op.kind == CoercionKind::Forward && !op.primary)
        .map(|(&id, op)| (id, op.owner, op.input))
        .collect();
    assert!(ids.len() >= 3);
    assert!(checker.coercions.values().any(|op| op.control));
    assert!(ids.iter().any(|(_, owner, _)| *owner != 0));
    for (id, owner, input) in ids {
        assert_eq!(
            checker
                .forward_coercion_input(&reports, id, owner, Span::default())
                .unwrap(),
            Some(input)
        );
    }
}

#[test]
pub(crate) fn forward_coercion_inputs_require_result_and_exclude_nontransparent_stages() {
    for source in [
        "n:7;a<int32>:n;b<int32><null>:n;r:{->7;->tag:true};c<int32>:r",
        "d:@\"debug\";f<boolean>:(r<{-><never>;tag<boolean>}>){->r};x<boolean>:d.panic(\"stop\")",
        "n:=7;p:&!n;q<&int32>:p;copy:*q",
    ] {
        let (mut checker, mut reports) = checked(source, false);
        let ids: Vec<_> = checker.coercions.keys().copied().collect();
        for id in ids {
            let op = &checker.coercions[&id];
            let (owner, input) = (op.owner, op.input);
            let expected = (op.kind == CoercionKind::Forward
                && !op.primary
                && reports.effects.contains_key(&id))
            .then_some(input);
            assert_eq!(
                checker
                    .forward_coercion_input(&reports, id, owner, Span::default())
                    .unwrap(),
                expected
            );
            if expected.is_some() {
                let (_, Effect::Coercion(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.result = false;
                assert_eq!(
                    checker
                        .forward_coercion_input(&reports, id, owner, Span::default())
                        .unwrap(),
                    None
                );
                reports.effects.remove(&id);
                assert_eq!(
                    checker
                        .forward_coercion_input(&reports, id, owner, Span::default())
                        .unwrap(),
                    None
                );
            }
        }
        for id in checker.reborrow_ops.keys().copied().collect::<Vec<_>>() {
            assert_eq!(
                checker
                    .forward_coercion_input(&reports, id, 0, Span::default())
                    .unwrap(),
                None
            );
        }
    }
}

#[test]
pub(crate) fn forward_coercion_inputs_reject_corrupt_headers_and_exact_edges_atomically() {
    for fault in 0..16 {
        let (mut checker, mut reports) = checked("n:7;x<int32>:n", false);
        let id = *checker.coercions.last_key_value().unwrap().0;
        let (reported_owner, Effect::Coercion(observed)) = reports.effects.get_mut(&id).unwrap()
        else {
            panic!()
        };
        let op = checker.coercions.get_mut(&id).unwrap();
        match fault {
            0 => *reported_owner = 9,
            1 => op.owner = 9,
            2 => observed.input = id,
            3 => observed.op = CoercionKind::Convert,
            4 => observed.primary = true,
            5 => observed.control = !observed.control,
            6 => observed.projected = true,
            7 => observed.operation = true,
            8 => op.edges.clear(),
            9 => op.edges[1].route = Route::Returned,
            10 => checker.points[op.input].parent = None,
            11 => checker.points[id].complete = false,
            12 => checker.points[op.input].span = Span::default(),
            13 => {
                checker.coercions.remove(&id);
            }
            14 => {
                observed.result = false;
                observed.control = !observed.control;
            }
            15 => {
                op.kind = CoercionKind::Convert;
                observed.op = op.kind;
                observed.input = id;
            }
            _ => unreachable!(),
        }
        let before = reports.effects.clone();
        let ops = checker.coercions.clone();
        let counts = checker.edge_counts();
        let error = checker
            .forward_coercion_input(&reports, id, 0, Span::default())
            .unwrap_err();
        assert!(
            error.message.contains("coercion-effect identity"),
            "fault {fault}"
        );
        assert_eq!(reports.effects, before);
        assert_eq!(checker.coercions, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn forward_coercion_inputs_bound_lookup_and_validation_work() {
    let (mut checker, reports) = checked("r<{n<int32>}>:(({->n:1}))", false);
    let (&id, op) = checker.coercions.last_key_value().unwrap();
    let input = op.input;
    let before = reports.effects.clone();
    let ops = checker.coercions.clone();
    let counts = checker.edge_counts();
    let start = checker.flow.work;
    assert_eq!(
        checker
            .forward_coercion_input(&reports, id, 0, Span::default())
            .unwrap(),
        Some(input)
    );
    let work = checker.flow.work - start;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.forward_coercion_input(&reports, id, 0, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(input) = result {
            assert!(input.is_some());
        }
        assert_eq!(reports.effects, before);
        assert_eq!(checker.coercions, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}
