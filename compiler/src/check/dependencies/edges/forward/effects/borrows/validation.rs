use super::{super::tests::checked, *};

#[test]
pub(crate) fn place_borrow_effect_validation_keeps_all_address_and_result_stages() {
    for mode in ["&", "&!"] {
        let source = format!("r:{{->a:0;->inner:{{->x:1;->y:=2;->z:3}}}};p:{mode}(r.inner.y)");
        let (mut checker, reports) = checked(&source, false);
        let id = *checker.place_borrows.first_key_value().unwrap().0;
        for port in (0..=2)
            .map(|step| Port::Address { point: id, step })
            .chain([Port::Operation(id), Port::Normal(id)])
        {
            assert!(
                checker
                    .validate_place_borrow(&reports, 0, port, Span::default())
                    .unwrap()
            );
        }
        for port in [Port::Entry(id), Port::Projection { point: id, step: 0 }] {
            assert!(
                !checker
                    .validate_place_borrow(&reports, 0, port, Span::default())
                    .unwrap()
            );
        }
        assert!(checker.locals.is_empty());
    }
}

#[test]
pub(crate) fn place_borrow_effect_validation_rejects_corrupt_identity_paths_and_edges() {
    for stage in 0..3 {
        for fault in 0..30 {
            let (mut checker, mut reports) =
                checked("r:{->a:0;->inner:{->x:1;->y:2}};p:&(r.inner.y)", false);
            let id = *checker.place_borrows.first_key_value().unwrap().0;
            let port = match stage {
                0 => Port::Address {
                    point: id,
                    step: usize::from(fault != 29),
                },
                1 => Port::Operation(id),
                _ => Port::Normal(id),
            };
            reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
            let before = reports.effects.clone();
            let op = checker.place_borrows.get_mut(&id).unwrap();
            match fault {
                0 => op.owner += 1,
                1 => checker.points[id].owner += 1,
                2 => checker.points[id].complete = false,
                3 => checker.points[id].kind = PointKind::Read,
                4 => checker.points[id].span = Span::default(),
                5 => op.span = Span::default(),
                6 => op.place.root = reports.locals,
                7 => op.storage = reports.locals,
                8 => op.storage += 1,
                9 => op.counts.clear(),
                10 => op.counts.push(3),
                11 => op.counts[0] = op.place.fields[0],
                12 => op.place.fields[1] = op.counts[1],
                13 => op.edges.clear(),
                14 => op.edges[0].from = Port::Normal(id),
                15 => op.edges[0].to = Port::Operation(id),
                16 => op.edges[1].from = Port::Entry(id),
                17 => op.edges[2].to = Port::Operation(id),
                18 => op.edges[3].to = Port::Normal(id),
                19 => op.edges[4].from = Port::Entry(id),
                20 => op.edges[4].route = Route::Returned,
                21 => op.edges.push(op.edges[4]),
                22 => op.edges.swap(0, 1),
                23 => {
                    reports.index.operations.remove(&id);
                }
                24 => {
                    reports.index.operations.insert(id, 9);
                }
                25 => checker.points[id].block = None,
                26 => checker.points[id].parent = Some(id),
                27 => checker.points[id].parent = Some(usize::MAX),
                28 => checker.points.truncate(id),
                29 => op.counts[1] = 0,
                _ => unreachable!(),
            }
            let ops = checker.place_borrows.clone();
            let counts = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(error.code, "B001", "stage {stage}, fault {fault}");
            assert!(error.message.contains("place-borrow-effect identity"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.place_borrows, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn place_borrow_effect_validation_bounds_selectors_paths_and_work() {
    let (mut checker, reports) = checked("r:{->x:1};p:&(r.x)", false);
    let id = *checker.place_borrows.first_key_value().unwrap().0;
    for step in [2, usize::MAX] {
        assert!(
            checker
                .validate_place_borrow(
                    &reports,
                    0,
                    Port::Address { point: id, step },
                    Span::default()
                )
                .is_err()
        );
    }
    let before = checker.flow.work;
    checker
        .validate_place_borrow(&reports, 0, Port::Normal(id), Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        assert_eq!(
            checker
                .validate_place_borrow(&reports, 0, Port::Normal(id), Span::default())
                .is_ok(),
            spare == 0
        );
    }
    checker.flow = crate::flow::Flow::default();
    checker.place_borrows.get_mut(&id).unwrap().place.fields =
        vec![0; crate::list::MAX_WRITE_PATH + 1];
    assert!(
        checker
            .validate_place_borrow(&reports, 0, Port::Normal(id), Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
}
