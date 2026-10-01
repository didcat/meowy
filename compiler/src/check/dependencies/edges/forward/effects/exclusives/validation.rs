use super::{super::tests::checked, *};

#[test]
pub(crate) fn exclusive_effect_validation_keeps_addresses_reservations_and_stopped_frontiers() {
    let source = "r:{->rows:=[{->xs:=[1]}]};p:&!(r.rows[1].xs[1])";
    let (mut checker, reports) = checked(source, false);
    let id = *checker.exclusives.first_key_value().unwrap().0;
    for port in (0..=3)
        .map(|step| Port::Address { point: id, step })
        .chain([
            Port::Reserve { point: id, step: 0 },
            Port::Reserve { point: id, step: 2 },
            Port::Operation(id),
            Port::Normal(id),
        ])
    {
        assert!(
            checker
                .validate_exclusive_borrow(&reports, 0, port, Span::default())
                .unwrap()
        );
    }
    assert!(
        checker
            .validate_exclusive_borrow(
                &reports,
                0,
                Port::Reserve { point: id, step: 1 },
                Span::default()
            )
            .is_err()
    );
    for (source, stop) in [
        ("xs:=[[1]];'out{p:&!(xs[{'out.leave()}][1])}", 0),
        ("xs:=[[1]];'out{p:&!(xs[1][{'out.leave()}])}", 1),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.exclusives.first_key_value().unwrap();
        assert_eq!(
            op.edges
                .iter()
                .filter(|edge| edge.route == Route::Checked)
                .count(),
            1
        );
        for step in 0..=2 {
            let result = checker.validate_exclusive_borrow(
                &reports,
                0,
                Port::Address { point: id, step },
                Span::default(),
            );
            assert_eq!(result.is_ok(), step <= stop);
            let result = checker.validate_exclusive_borrow(
                &reports,
                0,
                Port::Reserve { point: id, step },
                Span::default(),
            );
            assert_eq!(result.is_ok(), step <= stop);
        }
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(
                checker
                    .validate_exclusive_borrow(&reports, 0, port, Span::default())
                    .is_err()
            );
        }
    }
}

#[test]
pub(crate) fn exclusive_effect_validation_rejects_corrupt_sources_inputs_and_edges() {
    for stage in 0..4 {
        for fault in 0..35 {
            let (mut checker, mut reports) =
                checked("r:{->rows:=[{->xs:=[1]}]};p:&!(r.rows[1].xs[1])", false);
            let (&id, op) = checker.exclusives.first_key_value().unwrap();
            let PathStep::Index { point: child, .. } = op.steps[0] else {
                panic!()
            };
            let port = match stage {
                0 => Port::Address { point: id, step: 0 },
                1 => Port::Reserve { point: id, step: 0 },
                2 => Port::Operation(id),
                _ => Port::Normal(id),
            };
            reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
            let before = reports.effects.clone();
            let op = checker.exclusives.get_mut(&id).unwrap();
            let last = op.edges.len() - 1;
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
                9 => op.steps.clear(),
                10 => op.steps[0] = PathStep::Field(0),
                11 => op.counts.clear(),
                12 => op.place.fields[0] = op.counts[0],
                13 => op.steps[1] = PathStep::Field(1),
                14 => op.access.clear(),
                15 => op.access.push(op.access[0]),
                16 => op.access[0].length = Some(2),
                17 => op.access[0].normal = false,
                18 => op.normal = false,
                19 => {
                    let PathStep::Index { point, .. } = &mut op.steps[0] else {
                        panic!()
                    };
                    *point = usize::MAX;
                }
                20 => {
                    let PathStep::Index { point, .. } = &mut op.steps[2] else {
                        panic!()
                    };
                    *point = child;
                }
                21 => checker.points[child].owner += 1,
                22 => checker.points[child].parent = None,
                23 => checker.points[child].block = None,
                24 => checker.points[child].complete = false,
                25 => checker.points[child].kind = PointKind::Stmt,
                26 => {
                    let PathStep::Index { span, .. } = &mut op.steps[0] else {
                        panic!()
                    };
                    *span = Span::default();
                }
                27 => {
                    let PathStep::Index { capacity, .. } = &mut op.steps[0] else {
                        panic!()
                    };
                    *capacity = crate::list::MAX_CAPACITY + 1;
                }
                28 => op.edges.clear(),
                29 => op.edges[0].route = Route::Checked,
                30 => op.edges[last].route = Route::Checked,
                31 => op.edges.push(op.edges[last]),
                32 => {
                    reports.index.operations.remove(&id);
                }
                33 => {
                    reports.index.operations.insert(id, 9);
                }
                34 => op.counts.push(1),
                _ => unreachable!(),
            }
            let ops = checker.exclusives.clone();
            let counts = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(error.code, "B001", "stage {stage}, fault {fault}");
            assert!(error.message.contains("exclusive-effect"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.exclusives, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}
