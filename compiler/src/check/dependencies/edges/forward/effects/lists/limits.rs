use super::{super::tests::checked, *};

#[test]
pub(crate) fn list_effects_share_exact_work_effect_and_payload_limits() {
    let source = "f<int32>:(x<int32>){->x};xs:[f(1),2];r:{->n:=1};r.n=2";
    for (missing, parts, pass) in [(0, 8, true), (0, 7, false), (1, 8, false)] {
        let (mut checker, mut reports) = checked(source, false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let expected = reports.effects.clone();
        let lists = checker.lists.clone();
        let counts = checker.edge_counts();
        let limit = expected.len() - missing;
        let before = checker.flow.work;
        let result = checker.operation_effects_limited(&reports, Span::default(), limit, parts, 0);
        assert_eq!(result.is_ok(), pass);
        let work = checker.flow.work - before;
        if let Ok(actual) = result {
            assert_eq!(actual, expected);
            for spare in [0, 1] {
                checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
                let result =
                    checker.operation_effects_limited(&reports, Span::default(), limit, parts, 0);
                assert_eq!(result.is_ok(), spare == 0);
                if let Ok(actual) = result {
                    assert_eq!(actual, expected);
                }
            }
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.lists, lists);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn list_effects_preserve_partial_reports_on_conflicts_and_limits() {
    let source = "<T>:<int32><boolean>;<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<T[1]><string[1]>:[v]}";
    let (mut checker, _) = checked(source, false);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let port = Port::Projection { point: id, step: 0 };
    let mut effects = Effects::new();
    let mut parts = 2;
    assert!(
        checker
            .record_list_effect(owner, port, &mut effects, 1, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 2);
    parts = 3;
    assert!(
        checker
            .record_list_effect(owner, port, &mut effects, 0, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 3);
    checker
        .record_list_effect(owner, port, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(parts, 0);
    let expected = effects.clone();
    for fault in 0..10 {
        let mut effects = expected.clone();
        let (prior_owner, Effect::List(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *prior_owner += 1,
            1 => op.capacity += 1,
            2 => op.contextual = false,
            3 => op.normal = false,
            4 => op.control = true,
            5 => op.inputs.clear(),
            6 => op.inputs[0].point += 1,
            7 => op.inputs[0].plan = Some((false, CoercionKind::Convert)),
            8 => op.inputs[0].plan = Some((true, CoercionKind::Forward)),
            9 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_list_effect(
                    owner,
                    Port::Normal(id),
                    &mut effects,
                    1,
                    &mut parts,
                    Span::default()
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
        assert_eq!(parts, 0);
    }
    checker
        .record_list_effect(owner, port, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    assert_eq!(parts, 0);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_list_effect(
                owner,
                Port::Normal(id),
                &mut effects,
                1,
                &mut parts,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(effects, expected);
    assert_eq!(parts, 0);
}

#[test]
pub(crate) fn list_effects_bound_maximum_inputs_without_charging_unused_capacity() {
    let (mut checker, mut reports) = checked("xs<int32[65536]>:[]", false);
    let id = *checker.lists.first_key_value().unwrap().0;
    reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Operation(id), Port::Normal(id)];
    let effects = checker
        .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
        .unwrap();
    let (_, Effect::List(op)) = &effects[&id] else {
        panic!()
    };
    assert!(op.inputs.is_empty() && op.constructed && op.result);
    assert_eq!(op.capacity, crate::list::MAX_CAPACITY);
    let max = crate::list::MAX_CAPACITY;
    let mut items = Vec::with_capacity(max);
    for _ in 0..max {
        let child = checker.points.len();
        let point = &checker.points[id];
        checker.points.push(crate::check::dependencies::Point {
            kind: PointKind::Expr,
            owner: 0,
            block: point.block,
            site: point.site,
            parent: Some(id),
            span: point.span,
            complete: true,
        });
        items.push(Some(child));
    }
    let first = items[0].unwrap();
    let last = items[max - 1].unwrap();
    let key = SequenceSource::Expr(id);
    let sequence = checker.sequences.get_mut(&key).unwrap();
    sequence.edges = items
        .windows(2)
        .map(|pair| {
            Edge::new(
                Port::Normal(pair[0].unwrap()),
                Port::Entry(pair[1].unwrap()),
                Route::Next,
            )
        })
        .collect();
    sequence.items = items;
    checker.endpoints.insert(
        key,
        vec![
            Edge::new(Port::Entry(id), Port::Entry(first), Route::Next),
            Edge::new(Port::Normal(last), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ],
    );
    checker.lists.get_mut(&id).unwrap().count = max;
    reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Operation(id)];
    for missing in [0, 1] {
        checker.flow.work = 0;
        let result =
            checker.operation_effects_limited(&reports, Span::default(), 1, max * 3 - missing, 0);
        assert_eq!(result.is_ok(), missing == 0);
        if let Ok(effects) = result {
            let (_, Effect::List(op)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(op.inputs.len(), max);
            assert_eq!(op.inputs[0].point, first);
            assert_eq!(op.inputs[max - 1].point, last);
            assert!(op.constructed && !op.result);
        }
    }
    checker.lists.get_mut(&id).unwrap().count += 1;
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 1, usize::MAX, 0)
            .unwrap_err()
            .message
            .contains("budget")
    );
}
