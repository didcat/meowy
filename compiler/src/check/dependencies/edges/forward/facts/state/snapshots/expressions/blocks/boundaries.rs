use super::super::super::capture::tests::capture;
use super::*;

pub(super) fn setup(
    source: &str,
    check: impl FnOnce(
        &mut Checker,
        &mut Snapshots,
        PointId,
        observations::Forward,
        &mut observations::Observations<'_, '_>,
    ),
) {
    let (mut checker, program, reports, mut snapshots) = capture(source);
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (obs, parts) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            observations::Observations::new(&definitions),
            Span::default(),
            parts,
        )
        .unwrap();
    let (mut obs, _) = checker
        .scalar_slot_observations(&reports, &graph, obs, Span::default(), parts)
        .unwrap();
    let (&port, &forward) = obs.forwarded.iter().next().unwrap();
    let Port::Normal(point) = port else { panic!() };
    snapshots.parts += snapshots.expressions.remove(&point).unwrap().weight();
    check(&mut checker, &mut snapshots, point, forward, &mut obs);
}

#[test]
pub(crate) fn scalar_block_publication_reserves_exact_work_and_payload_atomically() {
    for source in ["v:{->7}", "v:{}"] {
        setup(source, |checker, snapshots, point, forward, obs| {
            let start = checker.flow.work;
            snapshots
                .block_expression(point, &forward, obs, &mut checker.flow, Span::default())
                .unwrap();
            let cost = checker.flow.work - start;
            let fact = snapshots.expressions.remove(&point).unwrap();
            for (room, extra, pass) in [
                (fact.weight() - 1, 0, false),
                (fact.weight(), 1, false),
                (fact.weight(), 0, true),
            ] {
                snapshots.parts = room;
                checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
                checker.flow.full = false;
                let saved = format!("{snapshots:?}");
                let result = snapshots.block_expression(
                    point,
                    &forward,
                    obs,
                    &mut checker.flow,
                    Span::default(),
                );
                assert_eq!(result.is_ok(), pass);
                if pass {
                    assert_eq!(snapshots.expressions[&point], fact);
                    assert_eq!(snapshots.parts, 0);
                } else {
                    assert_eq!(format!("{snapshots:?}"), saved);
                }
            }
            checker.flow.work = 0;
            checker.flow.full = false;
            snapshots.parts = MAX_EDGES;
            let saved = format!("{snapshots:?}");
            assert!(
                snapshots
                    .block_expression(point, &forward, obs, &mut checker.flow, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(format!("{snapshots:?}"), saved);
        });
    }
}

#[test]
pub(crate) fn scalar_block_publication_rejects_stale_headers_rows_and_shapes_atomically() {
    for fault in 0..14 {
        setup("v:{->7}", |checker, snapshots, point, mut forward, obs| {
            let Port::BlockResult(block) = forward.input else {
                panic!()
            };
            match fault {
                0 => obs.slots.get_mut(&block).unwrap().owner += 1,
                1 => obs.scopes.get_mut(&block).unwrap().parent = None,
                2 => obs.slots.get_mut(&block).unwrap().declared = Domain::boolean(true),
                3 => obs.scopes.get_mut(&block).unwrap().normal = false,
                4 => obs.scopes.get_mut(&block).unwrap().result = false,
                5 => forward.input = Port::Normal(point),
                6 => forward.owner += 1,
                7 => forward.control = true,
                8 => snapshots.rows.get_mut(&Port::Normal(point)).unwrap().before = Value::Missing,
                9 => snapshots.rows.get_mut(&forward.input).unwrap().before = Value::Missing,
                10 => {
                    snapshots
                        .rows
                        .get_mut(&Port::BlockNormal(block))
                        .unwrap()
                        .owner += 1
                }
                11 => snapshots.rows.get_mut(&forward.input).unwrap().control = None,
                12 => {
                    obs.scopes.get_mut(&block).unwrap().kind =
                        crate::check::dependencies::ScalarKind::Bool
                }
                13 => obs.slots.get_mut(&block).unwrap().control = Some(true),
                _ => unreachable!(),
            }
            let saved = format!("{snapshots:?}{obs:?}");
            assert!(
                snapshots
                    .block_expression(point, &forward, obs, &mut checker.flow, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "{fault}"
            );
            assert_eq!(format!("{snapshots:?}{obs:?}"), saved);
        });
    }
}

#[test]
pub(crate) fn scalar_block_publication_never_recovers_missing_history_from_a_dependent_slot() {
    for fault in 0..5 {
        setup("v:{->7}", |checker, snapshots, point, forward, obs| {
            let Port::BlockResult(block) = forward.input else {
                panic!()
            };
            let Value::Known(id) = snapshots.rows[&forward.input].after else {
                panic!()
            };
            let mut cell = snapshots.states[id].slot(block).unwrap().clone();
            cell.control = true;
            snapshots.states[id]
                .put_slot(block, &cell, &mut checker.flow, Span::default(), MAX_EDGES)
                .unwrap();
            match fault {
                0 => {
                    obs.scopes.remove(&block);
                }
                1 => {
                    obs.slots.remove(&block);
                }
                2 => {
                    snapshots.states[id] = State::new(
                        forward.owner,
                        crate::flow::TRUE,
                        &mut checker.flow,
                        Span::default(),
                    )
                    .unwrap()
                }
                3 | 4 => {
                    for port in [Port::BlockNormal(block), forward.input, Port::Normal(point)] {
                        let row = snapshots.rows.get_mut(&port).unwrap();
                        row.control = None;
                        if fault == 4 {
                            row.before = Value::Missing;
                            row.after = Value::Missing;
                        }
                    }
                }
                _ => unreachable!(),
            }
            snapshots
                .block_expression(point, &forward, obs, &mut checker.flow, Span::default())
                .unwrap();
            assert!(snapshots.expressions[&point].value.is_none(), "{fault}");
        });
    }
}

#[test]
pub(crate) fn scalar_block_publication_does_not_invent_values_for_incomplete_or_mismatched_slots() {
    for data in [
        Data::Uninitialized,
        Data::Partial,
        Data::Unavailable,
        Data::Initialized {
            version: None,
            domain: None,
            origin: None,
        },
        Data::Initialized {
            version: None,
            domain: Some(Domain::NULL),
            origin: None,
        },
    ] {
        setup("v:{->7}", |checker, snapshots, point, forward, obs| {
            let Port::BlockResult(block) = forward.input else {
                panic!()
            };
            let Value::Known(id) = snapshots.rows[&forward.input].after else {
                panic!()
            };
            snapshots.states[id]
                .put_slot(
                    block,
                    &Cell {
                        data,
                        control: false,
                    },
                    &mut checker.flow,
                    Span::default(),
                    MAX_EDGES,
                )
                .unwrap();
            snapshots
                .block_expression(point, &forward, obs, &mut checker.flow, Span::default())
                .unwrap();
            assert!(snapshots.expressions[&point].value.is_none());
        });
    }
}
