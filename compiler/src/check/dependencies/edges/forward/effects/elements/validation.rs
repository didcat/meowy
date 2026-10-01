use super::{super::tests::checked, *};

#[test]
pub(crate) fn element_effect_validation_keeps_owned_view_temporary_and_partial_stages() {
    for source in [
        "xs:[1];p:&(xs[1])",
        "xs:[1];v:&xs;p:&(v[1])",
        "x:*(&([1][1]))",
        "r:{->inner:{->xs:[1,2]}};p:&(r.inner.xs[2])",
        "rows:[{->xs:[1]}];n:*(&(rows[1].xs[1]))",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        assert!(checker.locals.is_empty());
        let ids = checker
            .elements
            .iter()
            .map(|(&id, op)| (id, op.owner))
            .collect::<Vec<_>>();
        assert!(!ids.is_empty());
        for (id, owner) in ids {
            for port in [
                Port::Address { point: id, step: 0 },
                Port::Operation(id),
                Port::Normal(id),
            ] {
                assert!(
                    checker
                        .validate_element_borrow(&reports, owner, port, Span::default())
                        .unwrap()
                );
            }
        }
    }
    let (mut checker, mut reports) = checked("xs:[1];'out{p:&(xs[{'out.leave()}])}", false);
    let id = *checker.elements.first_key_value().unwrap().0;
    let address = Port::Address { point: id, step: 0 };
    assert!(!reports.index.operations.contains_key(&id));
    assert!(
        checker
            .validate_element_borrow(&reports, 0, address, Span::default())
            .unwrap()
    );
    for port in [
        Port::Operation(id),
        Port::Normal(id),
        Port::Address { point: id, step: 1 },
    ] {
        assert!(
            checker
                .validate_element_borrow(&reports, 0, port, Span::default())
                .is_err()
        );
    }
    reports.index.operations.insert(id, 0);
    assert!(
        checker
            .validate_element_borrow(&reports, 0, address, Span::default())
            .is_err()
    );
}

#[test]
pub(crate) fn element_effect_validation_rejects_corrupt_roots_access_and_edges() {
    for stage in 0..3 {
        for fault in 0..34 {
            let (mut checker, mut reports) = checked("xs:[1];p:&(xs[1])", false);
            let (&id, op) = checker.elements.first_key_value().unwrap();
            let (parent, position) = (op.parent, op.access.unwrap().position);
            let port = match stage {
                0 => Port::Address { point: id, step: 0 },
                1 => Port::Operation(id),
                _ => Port::Normal(id),
            };
            reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
            let before = reports.effects.clone();
            let op = checker.elements.get_mut(&id).unwrap();
            match fault {
                0 => op.owner += 1,
                1 => checker.points[id].owner += 1,
                2 => checker.points[id].complete = false,
                3 => checker.points[id].kind = PointKind::Read,
                4 => checker.points[id].span = Span::default(),
                5 => op.span = Span::default(),
                6 => op.parent = usize::MAX,
                7 => op.parent = id,
                8 => op.access.as_mut().unwrap().position = parent,
                9 => op.access = None,
                10 => op.access.as_mut().unwrap().site = checker.reborrows,
                11 => op.access.as_mut().unwrap().capacity = crate::list::MAX_CAPACITY + 1,
                12 => op.access.as_mut().unwrap().length = Some(2),
                13 => op.access.as_mut().unwrap().may_return = false,
                14 => checker.points[parent].parent = None,
                15 => checker.points[parent].owner += 1,
                16 => checker.points[parent].block = None,
                17 => checker.points[parent].complete = false,
                18 => checker.points[parent].kind = PointKind::Stmt,
                19 => checker.points[position].parent = None,
                20 => checker.points[position].owner += 1,
                21 => checker.points[position].block = None,
                22 => checker.points[position].complete = false,
                23 => checker.points[position].kind = PointKind::Stmt,
                24 => op.edges.clear(),
                25 => op.edges[0].from = Port::Normal(id),
                26 => op.edges[1].to = Port::Normal(position),
                27 => op.edges[2].from = Port::Entry(parent),
                28 => op.edges[3].route = Route::Next,
                29 => op.edges[4].to = Port::Normal(parent),
                30 => op.edges.push(op.edges[4]),
                31 => {
                    reports.index.operations.remove(&id);
                }
                32 => {
                    reports.index.operations.insert(id, 9);
                }
                33 => op.source = ElementSource::Stopped,
                _ => unreachable!(),
            }
            let ops = checker.elements.clone();
            let counts = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(error.code, "B001", "stage {stage}, fault {fault}");
            assert!(error.message.contains("element-effect identity"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.elements, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}
