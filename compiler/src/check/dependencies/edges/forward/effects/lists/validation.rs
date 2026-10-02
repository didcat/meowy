use super::{super::tests::checked, *};

#[test]
pub(crate) fn list_effect_validation_keeps_empty_nested_and_contextual_stages() {
    for source in [
        "n<uint8>:2;xs:[1,n,3];empty<int32[4]>:[];nested:[[1],[2]];sum:1+2",
        "<T>:<int32><boolean>;<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<T[1]><string[1]>:[v]}",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let ids: Vec<_> = checker
            .lists
            .iter()
            .map(|(&id, op)| (id, op.owner))
            .collect();
        for (id, owner) in ids {
            let ports: Vec<_> = checker.endpoints[&SequenceSource::Expr(id)]
                .iter()
                .map(|edge| edge.to)
                .filter(|port| {
                    matches!(
                        port,
                        Port::Projection { .. }
                            | Port::Conversion { .. }
                            | Port::Operation(_)
                            | Port::Normal(_)
                    )
                })
                .collect();
            for port in ports {
                assert!(
                    checker
                        .validate_list_construction(&reports, owner, port, Span::default())
                        .unwrap()
                );
            }
        }
        for id in checker.binaries.keys().copied().collect::<Vec<_>>() {
            assert!(
                !checker
                    .validate_list_construction(&reports, 0, Port::Operation(id), Span::default())
                    .unwrap()
            );
        }
    }
}

#[test]
pub(crate) fn list_effect_validation_rejects_corrupt_headers_roots_slots_and_edges() {
    for fault in 0..27 {
        let (mut checker, mut reports) = checked("n<uint8>:2;xs:[1,n,3]", false);
        let id = *checker.lists.first_key_value().unwrap().0;
        let key = SequenceSource::Expr(id);
        let child = checker.sequences[&key].items[0].unwrap();
        let op = checker.lists.get_mut(&id).unwrap();
        match fault {
            0 => op.owner += 1,
            1 => op.capacity = 2,
            2 => op.capacity = crate::list::MAX_CAPACITY + 1,
            3 => op.count -= 1,
            4 => op.contextual = true,
            5 => op.normal = false,
            6 => op.span = Span::default(),
            7 => checker.points[id].owner += 1,
            8 => checker.points[id].complete = false,
            9 => checker.points[id].kind = PointKind::Read,
            10 => checker.points[id].span = Span::default(),
            11 => checker.sequences.get_mut(&key).unwrap().owner += 1,
            12 => checker.sequences.get_mut(&key).unwrap().items[0] = None,
            13 => checker.sequences.get_mut(&key).unwrap().items[1] = Some(child),
            14 => checker.sequences.get_mut(&key).unwrap().items[0] = Some(usize::MAX),
            15 => checker.sequences.get_mut(&key).unwrap().items.swap(0, 1),
            16 => checker.points[child].owner += 1,
            17 => checker.points[child].parent = None,
            18 => checker.points[child].block = None,
            19 => checker.points[child].complete = false,
            20 => checker.points[child].kind = PointKind::Stmt,
            21 => checker.sequences.get_mut(&key).unwrap().edges.clear(),
            22 => checker.endpoints.get_mut(&key).unwrap().clear(),
            23 => checker.endpoints.get_mut(&key).unwrap()[0].route = Route::Checked,
            24 => checker.endpoints.get_mut(&key).unwrap().push(Edge::new(
                Port::Entry(id),
                Port::Normal(id),
                Route::Next,
            )),
            25 => {
                reports.index.operations.remove(&id);
            }
            26 => {
                reports.index.operations.insert(id, 9);
            }
            _ => unreachable!(),
        }
        assert!(
            checker
                .validate_list_construction(&reports, 0, Port::Operation(id), Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
    }
}

#[test]
pub(crate) fn list_effect_validation_stops_before_direct_and_projected_never_suffixes() {
    for (source, primary) in [
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs<int32[2]><uint8[2]>:[stop(),{x:300;->x}]",
            false,
        ),
        (
            "<R>:<{-><never>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[2]><string[2]>:[v,{x:1;->x}]}",
            true,
        ),
    ] {
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.lists.first_key_value().unwrap();
        let owner = op.owner;
        for port in [
            Port::Projection { point: id, step: 0 },
            Port::Conversion { point: id, part: 0 },
            Port::Projection { point: id, step: 1 },
            Port::Operation(id),
            Port::Normal(id),
        ] {
            let result = checker.validate_list_construction(&reports, owner, port, Span::default());
            assert_eq!(
                result.is_ok(),
                primary && port == Port::Projection { point: id, step: 0 }
            );
        }
    }
}
