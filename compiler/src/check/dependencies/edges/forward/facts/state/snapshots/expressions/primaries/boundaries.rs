use super::super::super::capture::tests::capture;
use super::*;

pub(super) fn setup(
    source: &str,
    check: impl FnOnce(
        &mut Checker,
        &mut Snapshots,
        Port,
        observations::Primary,
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
        .scalar_observations(&reports, &graph, &definitions, Span::default(), parts)
        .unwrap();
    let (obs, parts) = checker
        .scalar_scope_observations(&reports, &graph, obs, Span::default(), parts)
        .unwrap();
    let (mut obs, _) = checker
        .scalar_slot_observations(&reports, &graph, obs, Span::default(), parts)
        .unwrap();
    let (&port, &primary) = obs.primaries.iter().next().unwrap();
    snapshots.parts += snapshots.projections.remove(&port).unwrap().weight();
    check(&mut checker, &mut snapshots, port, primary, &mut obs);
}

#[test]
pub(crate) fn primary_publication_rejects_stale_owners_rows_headers_and_ports_atomically() {
    for fault in 0..12 {
        setup(
            "-(({->7;->tag:false}))",
            |checker, snapshots, mut port, mut primary, obs| {
                match fault {
                    0 => primary.owner += 1,
                    1 => snapshots.rows.get_mut(&port).unwrap().owner += 1,
                    2 => snapshots.rows.get_mut(&port).unwrap().before = Value::Missing,
                    3 => {
                        snapshots
                            .rows
                            .get_mut(&Port::Normal(primary.input))
                            .unwrap()
                            .after = Value::Missing
                    }
                    4 => obs.scopes.get_mut(&primary.block).unwrap().owner += 1,
                    5 => obs.scopes.get_mut(&primary.block).unwrap().parent = None,
                    6 => {
                        obs.slots.get_mut(&primary.block).unwrap().declared = Domain::boolean(false)
                    }
                    7 => {
                        snapshots
                            .rows
                            .get_mut(&Port::BlockResult(primary.block))
                            .unwrap()
                            .control = None
                    }
                    8 => obs.scopes.get_mut(&primary.block).unwrap().result = false,
                    9 => {
                        obs.scopes.get_mut(&primary.block).unwrap().scalar.kind =
                            crate::check::dependencies::ScalarKind::Bool
                    }
                    10 => snapshots.rows.get_mut(&port).unwrap().control = None,
                    11 => port = Port::Normal(primary.input),
                    _ => unreachable!(),
                }
                let saved = format!("{snapshots:?}{obs:?}");
                assert!(
                    snapshots
                        .primary_expression(port, &primary, obs, &mut checker.flow, Span::default())
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
pub(crate) fn primary_publication_keeps_missing_and_unknown_histories_unavailable() {
    for fault in 0..6 {
        setup(
            "-(({->7;->tag:false}))",
            |checker, snapshots, port, primary, obs| {
                let parent = obs.scopes[&primary.block].parent.unwrap();
                match fault {
                    0 => {
                        obs.scopes.remove(&primary.block);
                    }
                    1 => {
                        obs.slots.remove(&primary.block);
                    }
                    2 => {
                        snapshots.rows.remove(&Port::Normal(primary.input));
                    }
                    3 => {
                        snapshots.rows.remove(&Port::Normal(parent));
                    }
                    4 | 5 => {
                        for row in snapshots.rows.values_mut() {
                            row.control = None;
                            if fault == 5 {
                                row.before = Value::Missing;
                                row.after = Value::Missing;
                            }
                        }
                    }
                    _ => unreachable!(),
                }
                snapshots
                    .primary_expression(port, &primary, obs, &mut checker.flow, Span::default())
                    .unwrap();
                assert!(snapshots.projections[&port].value.is_none(), "{fault}");
            },
        );
    }
}

#[test]
pub(crate) fn primary_publication_never_invents_missing_ambiguous_or_incompatible_slot_values() {
    for data in [
        Data::Uninitialized,
        Data::Partial,
        Data::Unavailable,
        Data::Conditional {
            domain: Domain::boolean(true),
            origin: None,
        },
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
        setup(
            "-(({->7;->tag:false}))",
            |checker, snapshots, port, primary, obs| {
                let Value::Known(id) = snapshots.rows[&port].after else {
                    panic!()
                };
                snapshots.states[id]
                    .put_slot(
                        primary.block,
                        &Cell {
                            data,
                            control: true,
                        },
                        &mut checker.flow,
                        Span::default(),
                        MAX_EDGES,
                    )
                    .unwrap();
                snapshots
                    .primary_expression(port, &primary, obs, &mut checker.flow, Span::default())
                    .unwrap();
                assert!(snapshots.projections[&port].value.is_none());
            },
        );
    }
}

#[test]
pub(crate) fn primary_publication_does_not_invent_unobserved_skipped_projections() {
    let (_, _, _, snapshots) = capture("false&&!({->true;->tag:7})");
    assert!(snapshots.projections.is_empty());
}
