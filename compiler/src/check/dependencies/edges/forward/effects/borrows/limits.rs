use super::{super::tests::checked, *};

#[test]
pub(crate) fn place_borrow_effects_share_exact_work_effect_and_payload_limits() {
    let source = "f<null>:(n<int32>){};r:{->inner:{->x:=1}};p:&(r.inner.x);n:*p;r.inner.x=2;f(n);@\"debug\".print(n)";
    crate::compile(source).unwrap();
    for (missing, parts, pass) in [(0, 19, true), (0, 18, false), (1, 19, false)] {
        let (mut checker, mut reports) = checked(source, false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let expected = reports.effects.clone();
        let ops = checker.place_borrows.clone();
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
        assert_eq!(checker.place_borrows, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn place_borrow_effects_preserve_partial_records_on_conflicts_and_limits() {
    let (mut checker, _) = checked("r:{->inner:{->x:=1}};p:&!(r.inner.x)", false);
    let id = *checker.place_borrows.first_key_value().unwrap().0;
    let mut effects = Effects::new();
    let mut parts = 4;
    assert!(
        checker
            .record_place_borrow_effect(
                0,
                Port::Operation(id),
                &mut effects,
                1,
                &mut parts,
                Span::default()
            )
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 4);
    parts = 5;
    assert!(
        checker
            .record_place_borrow_effect(
                0,
                Port::Operation(id),
                &mut effects,
                0,
                &mut parts,
                Span::default()
            )
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 5);
    let port = Port::Address { point: id, step: 1 };
    checker
        .record_place_borrow_effect(0, port, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(parts, 0);
    let expected = effects.clone();
    for fault in 0..8 {
        let mut effects = expected.clone();
        let (owner, Effect::Borrow(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.place.root += 1,
            2 => op.place.fields[0] += 1,
            3 => op.storage += 1,
            4 => op.mode = crate::hir::ReferenceMode::Shared,
            5 => op.control = true,
            6 => op.addresses.clear(),
            7 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_place_borrow_effect(
                    0,
                    Port::Normal(id),
                    &mut effects,
                    1,
                    &mut parts,
                    Span::default()
                )
                .is_err()
        );
        assert_eq!(effects, before);
        assert_eq!(parts, 0);
    }
    checker
        .record_place_borrow_effect(0, port, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_place_borrow_effect(
                0,
                Port::Normal(id),
                &mut effects,
                1,
                &mut parts,
                Span::default()
            )
            .is_err()
    );
    assert_eq!(effects, expected);
    assert_eq!(parts, 0);
}

#[test]
pub(crate) fn place_borrow_effects_bound_maximum_paths_and_address_flags() {
    let (mut checker, mut reports) = checked("r:{->x:1};p:&(r.x)", false);
    let (&id, op) = checker.place_borrows.first_key_value().unwrap();
    let before = reports.effects.clone();
    let mut op = op.clone();
    let len = crate::list::MAX_WRITE_PATH;
    op.place.fields = vec![0; len];
    op.counts = vec![1; len];
    let address = |step| Port::Address { point: id, step };
    op.edges = vec![Edge::new(Port::Entry(id), address(0), Route::Next)];
    op.edges
        .extend((0..len).map(|step| Edge::new(address(step), address(step + 1), Route::Next)));
    op.edges.extend([
        Edge::new(address(len), Port::Operation(id), Route::Next),
        Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
    ]);
    checker.place_borrows.insert(id, op);
    reports.entries.get_mut(&0).unwrap().1.ports = vec![address(len), address(len)];
    let effects = checker
        .operation_effects_limited(&reports, Span::default(), 1, len * 2 + 1, 0)
        .unwrap();
    let (_, Effect::Borrow(op)) = &effects[&id] else {
        panic!()
    };
    assert_eq!(op.addresses.len(), len + 1);
    assert_eq!(op.addresses.iter().filter(|&&seen| seen).count(), 1);
    assert!(op.addresses[len]);
    assert!(!op.acquired && !op.result);
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 1, len * 2, 0)
            .is_err()
    );
    assert_eq!(reports.effects, before);
}
