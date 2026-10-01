use super::{super::tests::checked, *};

#[test]
pub(crate) fn temporary_effect_validation_preserves_cells_sites_and_exact_roots() {
    for source in [
        "x:*(&7)",
        "x:*(&{->n:1})",
        "x:*(&[1,2])",
        "n:1;cell:&(&n)",
        "make<int32>:(){->7};x:*(&(make()))",
        "flag:false;|flag|x:*(&1);f:(){x:*(&2)}",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        assert!(checker.locals.is_empty());
        let ids = checker
            .temporary_borrows
            .iter()
            .map(|(&id, op)| (id, op.owner))
            .collect::<Vec<_>>();
        assert!(!ids.is_empty());
        for (id, owner) in ids {
            for port in [Port::Operation(id), Port::Normal(id)] {
                assert!(
                    checker
                        .validate_temporary_borrow(&reports, owner, port, Span::default())
                        .unwrap()
                );
            }
            assert!(
                !checker
                    .validate_temporary_borrow(&reports, owner, Port::Entry(id), Span::default())
                    .unwrap()
            );
        }
    }
}

#[test]
pub(crate) fn temporary_effect_validation_rejects_corrupt_cells_sites_and_edges() {
    for normal in [false, true] {
        for fault in 0..40 {
            let (mut checker, mut reports) = checked("x:*(&7)", false);
            let (&id, op) = checker.temporary_borrows.first_key_value().unwrap();
            let (input, cell) = (op.input, op.cell.unwrap());
            let root = checker.sites[&cell.statement].point.unwrap();
            reports.entries.get_mut(&0).unwrap().1.ports = vec![if normal {
                Port::Normal(id)
            } else {
                Port::Operation(id)
            }];
            let before = reports.effects.clone();
            let op = checker.temporary_borrows.get_mut(&id).unwrap();
            match fault {
                0 => op.owner += 1,
                1 => checker.points[id].owner += 1,
                2 => checker.points[id].complete = false,
                3 => checker.points[id].kind = PointKind::Read,
                4 => checker.points[id].span = Span::default(),
                5 => op.span = Span::default(),
                6 => op.input = usize::MAX,
                7 => op.input = id,
                8 => checker.points[input].parent = None,
                9 => checker.points[input].owner += 1,
                10 => checker.points[input].block = None,
                11 => checker.points[input].complete = false,
                12 => checker.points[input].kind = PointKind::Stmt,
                13 => op.cell = None,
                14 => op.cell.as_mut().unwrap().local = reports.locals,
                15 => op.cell.as_mut().unwrap().statement = checker.statements,
                16 => {
                    checker.proofs.temporaries.remove(&cell.local);
                }
                17 => {
                    checker.proofs.temporaries.insert(cell.local, usize::MAX);
                }
                18 => checker.points[id].site = None,
                19 => checker.points[input].site = None,
                20 => {
                    checker.sites.remove(&cell.statement);
                }
                21 => checker.sites.get_mut(&cell.statement).unwrap().complete = false,
                22 => checker.sites.get_mut(&cell.statement).unwrap().owner += 1,
                23 => checker.sites.get_mut(&cell.statement).unwrap().block = None,
                24 => checker.sites.get_mut(&cell.statement).unwrap().point = None,
                25 => checker.points[root].complete = false,
                26 => checker.points[root].owner += 1,
                27 => checker.points[root].kind = PointKind::Expr,
                28 => checker.points[root].site = None,
                29 => checker.points[root].span = Span::default(),
                30 => op.edges.clear(),
                31 => op.edges[0].from = Port::Normal(id),
                32 => op.edges[0].to = Port::Normal(input),
                33 => op.edges[1].from = Port::Entry(input),
                34 => op.edges[2].to = Port::Normal(input),
                35 => op.edges[2].route = Route::Checked,
                36 => op.edges.push(op.edges[2]),
                37 => op.edges.swap(0, 1),
                38 => {
                    reports.index.operations.remove(&id);
                }
                39 => {
                    reports.index.operations.insert(id, 9);
                }
                _ => unreachable!(),
            }
            let ops = checker.temporary_borrows.clone();
            let counts = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(error.code, "B001", "fault {fault}, normal {normal}");
            assert!(error.message.contains("temporary-effect identity"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.temporary_borrows, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}
