use super::{super::tests::checked, *};

#[test]
pub(crate) fn projection_effects_reject_corrupt_materialized_cells_and_statement_roots() {
    for normal in [false, true] {
        for fault in 0..18 {
            let (mut checker, mut reports) = checked("x:*(&({->n:1}.n))", false);
            let (&id, op) = checker.projections.first_key_value().unwrap();
            let parent = op.parent;
            let ProjectionStep::Materialize { local, statement } = op.steps[0] else {
                panic!()
            };
            let root = checker.sites[&statement].point.unwrap();
            reports.entries.get_mut(&0).unwrap().1.ports = vec![if normal {
                Port::Normal(id)
            } else {
                Port::Projection { point: id, step: 0 }
            }];
            match fault {
                0 => {
                    checker.projections.get_mut(&id).unwrap().steps[0] =
                        ProjectionStep::Materialize {
                            local: reports.locals,
                            statement,
                        }
                }
                1 => {
                    checker.projections.get_mut(&id).unwrap().steps[0] =
                        ProjectionStep::Materialize {
                            local,
                            statement: checker.statements,
                        }
                }
                2 => {
                    checker.proofs.temporaries.remove(&local);
                }
                3 => {
                    checker.proofs.temporaries.insert(local, usize::MAX);
                }
                4 => checker.points[id].site = None,
                5 => checker.points[parent].site = None,
                6 => {
                    checker.sites.remove(&statement);
                }
                7 => checker.sites.get_mut(&statement).unwrap().owner += 1,
                8 => checker.sites.get_mut(&statement).unwrap().complete = false,
                9 => checker.sites.get_mut(&statement).unwrap().block = None,
                10 => checker.sites.get_mut(&statement).unwrap().point = None,
                11 => checker.points[root].complete = false,
                12 => checker.points[root].kind = PointKind::Expr,
                13 => checker.points[root].owner += 1,
                14 => checker.points[root].site = None,
                15 => checker.points[root].span = Span::default(),
                16 => checker.points[root].block = None,
                17 => checker
                    .projections
                    .get_mut(&id)
                    .unwrap()
                    .steps
                    .insert(1, ProjectionStep::Materialize { local, statement }),
                _ => unreachable!(),
            }
            let before = reports.effects.clone();
            let ops = checker.projections.clone();
            let counts = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert!(
                error.message.contains("projection-effect identity"),
                "fault {fault}, normal {normal}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.projections, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}
