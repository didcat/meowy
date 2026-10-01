use super::{super::tests::checked, *};

#[test]
pub(crate) fn projection_effect_validation_keeps_fields_loads_temporaries_and_conversions() {
    for source in [
        "r:{->n:1};h:{->p:&r};q:&(h.p.n)",
        "r:{->n:1};h:{->p:&r};v:&h;q:&(v.p.n)",
        "x:*(&({->n:1}.n))",
        "rows:[{->n:1}];q:&(rows[1].n)",
        "<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p<&R><null>:&r};|h.p<&R>|q:&(h.p.n)",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.projections.first_key_value().unwrap();
        let owner = op.owner;
        let mut ports = vec![Port::Operation(id), Port::Normal(id)];
        for (step, item) in op.steps.iter().enumerate() {
            ports.push(Port::Projection { point: id, step });
            if matches!(item, ProjectionStep::Field { narrow: true, .. }) {
                ports.push(Port::Conversion {
                    point: id,
                    part: step,
                });
            }
        }
        for port in ports {
            assert!(
                checker
                    .validate_borrow_projection(&reports, owner, port, Span::default())
                    .unwrap()
            );
        }
        for port in [
            Port::Projection {
                point: id,
                step: usize::MAX,
            },
            Port::Conversion {
                point: id,
                part: usize::MAX,
            },
        ] {
            assert!(
                checker
                    .validate_borrow_projection(&reports, owner, port, Span::default())
                    .is_err()
            );
        }
    }
}

#[test]
pub(crate) fn projection_effect_validation_rejects_corrupt_identity_steps_and_edges() {
    let source = "<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p<&R><null>:&r};|h.p<&R>|q:&(h.p.n)";
    for stage in 0..4 {
        for fault in 0..32 {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, op) = checker.projections.first_key_value().unwrap();
            let parent = op.parent;
            let port = match stage {
                0 => Port::Projection { point: id, step: 0 },
                1 => Port::Conversion { point: id, part: 0 },
                2 => Port::Operation(id),
                _ => Port::Normal(id),
            };
            reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
            let before = reports.effects.clone();
            let op = checker.projections.get_mut(&id).unwrap();
            let last = op.edges.len() - 1;
            match fault {
                0 => op.owner += 1,
                1 => checker.points[id].owner += 1,
                2 => checker.points[id].complete = false,
                3 => checker.points[id].kind = PointKind::Read,
                4 => checker.points[id].span = Span::default(),
                5 => op.span = Span::default(),
                6 => op.parent = usize::MAX,
                7 => op.parent = id,
                8 => checker.points[parent].owner += 1,
                9 => checker.points[parent].parent = None,
                10 => checker.points[parent].block = None,
                11 => checker.points[parent].complete = false,
                12 => checker.points[parent].kind = PointKind::Stmt,
                13 => op.site = None,
                14 => op.mode = None,
                15 => op.site = Some(checker.reborrows),
                16 => {
                    let ProjectionStep::Field { count, .. } = &mut op.steps[0] else {
                        panic!()
                    };
                    *count = 0;
                }
                17 => {
                    let ProjectionStep::Address { count, .. } = &mut op.steps[1] else {
                        panic!()
                    };
                    *count = 0;
                }
                18 => op.steps.push(op.steps[0].clone()),
                19 => op
                    .steps
                    .insert(0, ProjectionStep::Load(crate::hir::ReferenceMode::Shared)),
                20 => {
                    op.steps[0] = ProjectionStep::Field {
                        index: crate::borrow_value::MAX_PARTS,
                        count: usize::MAX,
                        narrow: true,
                    }
                }
                21 => {
                    op.steps[1] = ProjectionStep::Address {
                        index: crate::borrow_value::MAX_PARTS,
                        count: usize::MAX,
                    }
                }
                22 => op.edges.clear(),
                23 => op.edges[0].from = Port::Normal(id),
                24 => op.edges[1].to = Port::Projection { point: id, step: 1 },
                25 => op.edges[2].from = Port::Normal(parent),
                26 => op.edges[last].to = Port::Normal(parent),
                27 => op.edges[last].route = Route::Checked,
                28 => op.edges.push(op.edges[last]),
                29 => {
                    reports.index.operations.remove(&id);
                }
                30 => {
                    reports.index.operations.insert(id, 9);
                }
                31 => {
                    let ProjectionStep::Field { narrow, .. } = &mut op.steps[0] else {
                        panic!()
                    };
                    *narrow = false;
                }
                _ => unreachable!(),
            }
            let ops = checker.projections.clone();
            let counts = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert_eq!(error.code, "B001", "stage {stage}, fault {fault}");
            assert!(error.message.contains("projection-effect identity"));
            assert_eq!(reports.effects, before);
            assert_eq!(checker.projections, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}
