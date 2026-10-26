use super::super::super::primaries::boundaries::setup;
use super::*;

pub(super) fn read(obs: &observations::Observations<'_, '_>) -> PointId {
    obs.sources
        .iter()
        .filter_map(|(&port, source)| match port {
            Port::Normal(point)
                if obs.records.contains_key(&point) && matches!(source.1, Effect::Read { .. }) =>
            {
                Some(point)
            }
            _ => None,
        })
        .max()
        .unwrap()
}

#[test]
pub(crate) fn record_read_publication_rejects_stale_state_definition_and_block_identity() {
    for fault in 0..5 {
        setup(
            "x:=7;r:{->x;->tag:false};v<int32>:r",
            |checker, snapshots, _, primary, obs| {
                let point = read(obs);
                snapshots.records.remove(&point).unwrap();
                let (_, Effect::Read { local, .. }) = obs.sources[&Port::Normal(point)] else {
                    panic!()
                };
                let location = obs.definitions.declarations[local];
                let mut block = primary.block;
                match fault {
                    0 => {
                        snapshots
                            .rows
                            .get_mut(&Port::Operation(point))
                            .unwrap()
                            .owner += 1
                    }
                    1 => {
                        snapshots.rows.get_mut(&Port::Normal(point)).unwrap().before =
                            Value::Missing
                    }
                    2 => {
                        obs.declared.insert(*local, Domain::NULL);
                    }
                    3 => snapshots.rows.get_mut(&location.port).unwrap().after = Value::Missing,
                    4 => block = usize::MAX,
                    _ => unreachable!(),
                }
                let saved = format!("{snapshots:?}{obs:?}");
                assert!(
                    snapshots
                        .record_expression_value(
                            point,
                            block,
                            obs,
                            &mut checker.flow,
                            Span::default()
                        )
                        .unwrap_err()
                        .message
                        .contains("identity"),
                    "{fault}"
                );
                assert_eq!(format!("{snapshots:?}{obs:?}"), saved);
            },
        );
    }
}

#[test]
pub(crate) fn record_read_publication_keeps_missing_operations_and_unknown_cells_unavailable() {
    for fault in 0..5 {
        setup(
            "x:=7;r:{->x;->tag:false};v<int32>:r",
            |checker, snapshots, _, primary, obs| {
                let point = read(obs);
                snapshots.records.remove(&point).unwrap();
                match fault {
                    0 => {
                        snapshots.rows.remove(&Port::Operation(point));
                    }
                    1 => {
                        for port in [Port::Operation(point), Port::Normal(point)] {
                            snapshots.rows.get_mut(&port).unwrap().control = None;
                        }
                    }
                    2..=4 => {
                        let Value::Known(id) = snapshots.rows[&Port::Normal(point)].after else {
                            panic!()
                        };
                        let (_, Effect::Read { local, .. }) = obs.sources[&Port::Normal(point)]
                        else {
                            panic!()
                        };
                        let data = match fault {
                            2 => Data::Partial,
                            3 => Data::Unavailable,
                            _ => Data::Initialized {
                                version: None,
                                domain: Some(Domain::NULL),
                                origin: None,
                            },
                        };
                        snapshots.states[id]
                            .put(
                                *local,
                                &Cell {
                                    data,
                                    control: true,
                                },
                                &mut checker.flow,
                                Span::default(),
                                MAX_EDGES,
                            )
                            .unwrap();
                    }
                    _ => unreachable!(),
                }
                snapshots
                    .record_expression_value(
                        point,
                        primary.block,
                        obs,
                        &mut checker.flow,
                        Span::default(),
                    )
                    .unwrap();
                assert!(snapshots.records[&point].1.value.is_none(), "{fault}");
            },
        );
    }
}

#[test]
pub(crate) fn record_read_publication_reserves_exact_budgets_and_rejects_duplicates_atomically() {
    setup(
        "x:=7;r:{->x;->tag:false};v<int32>:r",
        |checker, snapshots, _, primary, obs| {
            let point = read(obs);
            snapshots.parts += 1 + snapshots.records.remove(&point).unwrap().1.weight();
            let start = checker.flow.work;
            snapshots
                .record_expression_value(
                    point,
                    primary.block,
                    obs,
                    &mut checker.flow,
                    Span::default(),
                )
                .unwrap();
            let cost = checker.flow.work - start;
            let record = snapshots.records.remove(&point).unwrap();
            let used = 1 + record.1.weight();
            for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
                snapshots.parts = room;
                checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
                checker.flow.full = false;
                let saved = format!("{snapshots:?}");
                let result = snapshots.record_expression_value(
                    point,
                    primary.block,
                    obs,
                    &mut checker.flow,
                    Span::default(),
                );
                assert_eq!(result.is_ok(), pass);
                if pass {
                    assert_eq!(snapshots.records[&point], record);
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
                    .record_expression_value(
                        point,
                        primary.block,
                        obs,
                        &mut checker.flow,
                        Span::default()
                    )
                    .is_err()
            );
            assert_eq!(format!("{snapshots:?}"), saved);
        },
    );
}
