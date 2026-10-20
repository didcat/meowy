use super::*;

#[test]
pub(crate) fn scalar_state_sharing_bounds_a_thousand_updates_and_branch_joins() {
    let mut flow = Flow::new();
    let cells = (0..1000)
        .map(|local| {
            (
                local,
                Cell {
                    data: Data::Uninitialized,
                    control: false,
                },
            )
        })
        .collect();
    let mut state =
        State::from_sorted(0, TRUE, cells, &mut flow, Span::default(), MAX_EDGES).unwrap();
    let mut used = state.allocation();
    let mut history = Vec::new();
    for local in 0..1000 {
        let mut next = state
            .snapshot(&mut flow, Span::default(), MAX_EDGES - used)
            .unwrap();
        next.put(
            local,
            &Cell {
                data: Data::Initialized {
                    version: Some(Version {
                        owner: 0,
                        local,
                        definition: Definition::Bind(local),
                    }),
                    domain: Some(Domain::boolean(true)),
                    origin: Some(Origin::Expression(0, local)),
                },
                control: false,
            },
            &mut flow,
            Span::default(),
            MAX_EDGES - used,
        )
        .unwrap();
        used += next.allocation();
        let joined = state
            .join(&next, &mut flow, Span::default(), MAX_EDGES - used)
            .unwrap();
        used += joined.allocation();
        history.push(next);
        state = joined;
    }
    assert!(used < MAX_EDGES / 4);
    assert_eq!(state.get(0).unwrap().data, Data::Partial);
    assert!(matches!(
        history[0].get(0).unwrap().data,
        Data::Initialized { .. }
    ));
    assert_eq!(history[0].get(999).unwrap().data, Data::Uninitialized);
    assert_eq!(
        state
            .snapshot(&mut flow, Span::default(), 1)
            .unwrap()
            .allocation(),
        1
    );
}
use crate::flow::TRUE;
use versions::Definition;

#[test]
pub(crate) fn scalar_version_origins_preserve_copy_identity_across_equal_value_replacements() {
    let mut flow = Flow::new();
    let mut state = State::new(0, TRUE, &mut flow, Span::default()).unwrap();
    let version = Version {
        owner: 0,
        local: 0,
        definition: Definition::Write(1),
    };
    let mut cell = Cell {
        data: Data::Initialized {
            version: Some(version),
            domain: Some(Domain::boolean(true)),
            origin: Some(Origin::of_version(version, Span::default()).unwrap()),
        },
        control: false,
    };
    state
        .put(0, &cell, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let Data::Initialized { version, .. } = &mut cell.data else {
        panic!()
    };
    *version = Some(Version {
        owner: 0,
        local: 1,
        definition: Definition::Bind(2),
    });
    state
        .put(1, &cell, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let version = Version {
        owner: 0,
        local: 0,
        definition: Definition::Write(3),
    };
    let replacement = Cell {
        data: Data::Initialized {
            version: Some(version),
            domain: Some(Domain::boolean(true)),
            origin: Some(Origin::of_version(version, Span::default()).unwrap()),
        },
        control: false,
    };
    state
        .put(0, &replacement, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(state.get(1), Some(&cell));
    assert_ne!(state.get(0), state.get(1));
    let foreign = Cell {
        data: Data::Initialized {
            version: None,
            domain: Some(Domain::boolean(true)),
            origin: Some(Origin::Version(Version {
                owner: 1,
                ..version
            })),
        },
        control: false,
    };
    assert!(
        state
            .put(0, &foreign, &mut flow, Span::default(), MAX_EDGES)
            .is_err()
    );
    assert!(
        Origin::of_version(
            Version {
                owner: 0,
                local: 0,
                definition: Definition::Parameter(0)
            },
            Span::default()
        )
        .is_err()
    );
}

pub(super) fn value(local: usize, point: PointId, n: i128, flow: &mut Flow) -> Cell {
    Cell {
        data: Data::Initialized {
            version: Some(Version {
                owner: 0,
                local,
                definition: Definition::Bind(point),
            }),
            domain: Some(Domain::integer(n, flow, Span::default()).unwrap()),
            origin: Some(Origin::Expression(0, point)),
        },
        control: false,
    }
}

#[test]
pub(crate) fn scalar_state_separates_unavailable_partial_uninitialized_and_full_domains() {
    let mut flow = Flow::new();
    let mut state = State::new(0, TRUE, &mut flow, Span::default()).unwrap();
    for (local, data) in [
        Data::Unavailable,
        Data::Uninitialized,
        Data::Partial,
        Data::Initialized {
            version: None,
            domain: None,
            origin: None,
        },
        Data::Initialized {
            version: None,
            domain: Some(
                Domain::boolean(true)
                    .join(&Domain::boolean(false), &mut flow, Span::default())
                    .unwrap(),
            ),
            origin: None,
        },
    ]
    .into_iter()
    .enumerate()
    {
        state
            .put(
                local,
                &Cell {
                    data,
                    control: false,
                },
                &mut flow,
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
    }
    assert!(state.get(9).is_none());
    assert_ne!(state.reach, FALSE);
    for local in 0..4 {
        assert_ne!(state.get(local), state.get(local + 1));
    }
    let cell = Cell {
        data: Data::Initialized {
            version: None,
            domain: Some(Domain::BOTTOM),
            origin: None,
        },
        control: false,
    };
    state
        .put(5, &cell, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(state.reach, FALSE);
    assert_eq!(state.weight(), 1);
    assert!(state.get(0).is_none());
}

#[test]
pub(crate) fn scalar_state_snapshots_and_copies_keep_historical_values() {
    let mut flow = Flow::new();
    let mut state = State::new(0, TRUE, &mut flow, Span::default()).unwrap();
    state
        .put(
            0,
            &value(0, 1, 7, &mut flow),
            &mut flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let before = state
        .snapshot(&mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let mut copy = state.get(0).unwrap().clone();
    let Data::Initialized { version, .. } = &mut copy.data else {
        panic!()
    };
    *version = Some(Version {
        owner: 0,
        local: 1,
        definition: Definition::Bind(2),
    });
    state
        .put(1, &copy, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    state
        .put(
            0,
            &value(0, 3, 9, &mut flow),
            &mut flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert_ne!(state.get(0), before.get(0));
    assert_eq!(state.get(1), Some(&copy));
    assert!(before.get(1).is_none());
}

#[test]
pub(crate) fn scalar_state_updates_bound_payload_and_reject_foreign_identity_atomically() {
    let mut flow = Flow::new();
    let mut state = State::new(0, TRUE, &mut flow, Span::default()).unwrap();
    let cell = value(0, 1, 7, &mut flow);
    let cost = 2 + cell.weight();
    let before = state
        .snapshot(&mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(
        state
            .put(0, &cell, &mut flow, Span::default(), cost - 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(state, before);
    assert!(
        state
            .put(1, &cell, &mut flow, Span::default(), cost)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(state, before);
    state
        .put(0, &cell, &mut flow, Span::default(), cost)
        .unwrap();
    assert_eq!(state.weight(), cost);
    assert!(
        state
            .snapshot(&mut flow, Span::default(), 0)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(
        state,
        state.snapshot(&mut flow, Span::default(), 1).unwrap()
    );
}

#[test]
pub(crate) fn scalar_state_sorted_construction_validates_every_row_and_full_payload() {
    let mut flow = Flow::new();
    let cell = Cell {
        data: Data::Uninitialized,
        control: false,
    };
    let rows = vec![(0, cell.clone()), (1, cell.clone())];
    let state = State::from_sorted(0, TRUE, rows.clone(), &mut flow, Span::default(), 6).unwrap();
    assert_eq!(state.weight(), 5);
    assert_eq!(state.allocation(), 6);
    assert!(
        State::from_sorted(0, TRUE, rows, &mut flow, Span::default(), 5)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for keys in [[1, 0], [0, 0]] {
        let rows = keys
            .into_iter()
            .map(|local| (local, cell.clone()))
            .collect();
        assert!(
            State::from_sorted(0, FALSE, rows, &mut flow, Span::default(), 5)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    let invalid = value(1, 7, 1, &mut flow);
    assert!(
        State::from_sorted(
            0,
            TRUE,
            vec![(0, invalid)],
            &mut flow,
            Span::default(),
            MAX_EDGES
        )
        .unwrap_err()
        .message
        .contains("identity")
    );
    let dead = Cell {
        data: Data::Initialized {
            version: None,
            domain: Some(Domain::BOTTOM),
            origin: None,
        },
        control: false,
    };
    let rows = vec![(0, dead), (1, value(0, 7, 1, &mut flow))];
    assert!(
        State::from_sorted(0, TRUE, rows, &mut flow, Span::default(), MAX_EDGES)
            .unwrap_err()
            .message
            .contains("identity")
    );
}
