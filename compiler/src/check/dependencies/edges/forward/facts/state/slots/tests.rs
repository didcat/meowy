use super::*;
use crate::flow::TRUE;

pub(super) fn value(number: i128, point: usize, flow: &mut Flow) -> Cell {
    Cell {
        data: Data::Initialized {
            version: None,
            domain: Some(Domain::integer(number, flow, Span::default()).unwrap()),
            origin: Some(Origin::Expression(0, point)),
        },
        control: false,
    }
}

#[test]
pub(crate) fn scalar_slot_states_separate_local_keys_and_preserve_earlier_copies() {
    let mut flow = Flow::new();
    let mut state = State::new(0, crate::flow::TRUE, &mut flow, Span::default()).unwrap();
    let local = value(7, 1, &mut flow);
    state
        .put(0, &local, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    state
        .put_slot(
            0,
            &Cell {
                data: Data::Uninitialized,
                control: false,
            },
            &mut flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let old = state
        .snapshot(&mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let slot = value(9, 2, &mut flow);
    state
        .put_slot(0, &slot, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(state.get(0), Some(&local));
    assert_eq!(old.get(0), Some(&local));
    assert_eq!(state.slot(0), Some(&slot));
    assert_eq!(old.slot(0).unwrap().data, Data::Uninitialized);
    assert!(state.weight() > old.weight());
}

#[test]
pub(crate) fn scalar_slot_states_join_initialized_partial_missing_and_bottom_paths() {
    let mut flow = Flow::new();
    let mut empty = State::new(0, crate::flow::TRUE, &mut flow, Span::default()).unwrap();
    empty
        .put_slot(
            0,
            &Cell {
                data: Data::Uninitialized,
                control: false,
            },
            &mut flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let mut left = empty
        .snapshot(&mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let mut right = empty
        .snapshot(&mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    left.put_slot(
        0,
        &value(7, 1, &mut flow),
        &mut flow,
        Span::default(),
        MAX_EDGES,
    )
    .unwrap();
    let mut changed = value(9, 2, &mut flow);
    changed.control = true;
    right
        .put_slot(0, &changed, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let joined = left
        .join(&right, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let cell = joined.slot(0).unwrap();
    assert!(cell.control);
    let Data::Initialized {
        version,
        origin,
        domain: Some(domain),
    } = &cell.data
    else {
        panic!()
    };
    assert_eq!((*version, *origin), (None, None));
    assert_eq!(
        *domain,
        Domain::integer(7, &mut flow, Span::default())
            .unwrap()
            .join(
                &Domain::integer(9, &mut flow, Span::default()).unwrap(),
                &mut flow,
                Span::default(),
            )
            .unwrap()
    );
    assert_eq!(
        left.join(&empty, &mut flow, Span::default(), MAX_EDGES)
            .unwrap()
            .slot(0)
            .unwrap()
            .data,
        Data::Conditional {
            domain: Domain::integer(7, &mut flow, Span::default()).unwrap(),
            origin: Some(Origin::Expression(0, 1)),
        }
    );
    let absent = State::new(0, crate::flow::TRUE, &mut flow, Span::default()).unwrap();
    assert_eq!(
        left.join(&absent, &mut flow, Span::default(), MAX_EDGES)
            .unwrap()
            .slot(0)
            .unwrap()
            .data,
        Data::Unavailable
    );
    let bottom = right
        .restrict(FALSE, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(bottom.slot(0).is_none());
    assert_eq!(
        left.join(&bottom, &mut flow, Span::default(), MAX_EDGES)
            .unwrap(),
        left
    );
}

#[test]
pub(crate) fn scalar_slot_states_reserve_combined_payload_and_reject_invalid_identity() {
    let mut flow = Flow::new();
    let mut state = State::new(0, crate::flow::TRUE, &mut flow, Span::default()).unwrap();
    state
        .put(
            0,
            &value(7, 1, &mut flow),
            &mut flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let cell = value(9, 2, &mut flow);
    let mut attempt = state
        .snapshot(&mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let start = flow.work;
    attempt
        .put_slot(0, &cell, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = flow.work - start;
    let used = attempt.allocation();
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        flow.work = 0;
        flow.full = false;
        let mut attempt = state
            .snapshot(&mut flow, Span::default(), MAX_EDGES)
            .unwrap();
        let before = format!("{attempt:?}");
        flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        flow.full = false;
        let result = attempt.put_slot(0, &cell, &mut flow, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if !pass {
            assert_eq!(format!("{attempt:?}"), before);
        }
    }
    for version in [false, true] {
        flow.work = 0;
        flow.full = false;
        let mut cell = cell.clone();
        let Data::Initialized {
            version: stored,
            origin,
            ..
        } = &mut cell.data
        else {
            panic!()
        };
        if version {
            *stored = Some(Version {
                owner: 0,
                local: 0,
                definition: versions::Definition::Bind(0),
            });
        } else {
            *origin = Some(Origin::Expression(1, 2));
        }
        let before = format!("{state:?}");
        assert!(
            state
                .put_slot(0, &cell, &mut flow, Span::default(), MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{state:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_slot_states_never_reinitialize_known_or_uncertain_cells() {
    let mut flow = Flow::new();
    let slot = observations::Slot {
        owner: 0,
        declared: Domain::scalar(
            crate::check::dependencies::ScalarKind::Int {
                bits: 32,
                signed: true,
            },
            &mut flow,
            Span::default(),
        )
        .unwrap()
        .unwrap(),
        control: Some(false),
    };
    let old = value(7, 1, &mut flow);
    for data in [
        None,
        Some(Data::Partial),
        Some(Data::Unavailable),
        Some(old.data.clone()),
    ] {
        let mut state = State::new(0, crate::flow::TRUE, &mut flow, Span::default()).unwrap();
        if let Some(data) = data {
            state
                .put_slot(
                    0,
                    &Cell {
                        data,
                        control: true,
                    },
                    &mut flow,
                    Span::default(),
                    MAX_EDGES,
                )
                .unwrap();
        }
        let saved = format!("{state:?}");
        let cell = value(9, 2, &mut flow);
        assert!(
            state
                .initialize_slot(0, &slot, cell, &mut flow, Span::default(), MAX_EDGES)
                .unwrap()
                .is_none()
        );
        assert_eq!(format!("{state:?}"), saved);
    }
}

pub(super) fn frame(cell: Option<&Cell>, flow: &mut Flow) -> State {
    let mut state = State::new(0, TRUE, flow, Span::default()).unwrap();
    if let Some(cell) = cell {
        state
            .put_slot(0, cell, flow, Span::default(), MAX_EDGES)
            .unwrap();
    }
    state
}

#[test]
pub(crate) fn scalar_slot_joins_retain_only_initialized_alternatives_and_agreed_origins() {
    let mut flow = Flow::new();
    let empty = Cell {
        data: Data::Uninitialized,
        control: false,
    };
    let first = value(7, 1, &mut flow);
    let mut second = value(9, 1, &mut flow);
    second.control = true;
    let mut a = frame(Some(&first), &mut flow);
    let mut b = frame(Some(&empty), &mut flow);
    a.put(0, &first, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    b.put(0, &empty, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let partial = a.join(&b, &mut flow, Span::default(), MAX_EDGES).unwrap();
    assert_eq!(partial.get(0).unwrap().data, Data::Partial);
    assert_eq!(
        partial.slot(0).unwrap().data,
        Data::Conditional {
            domain: Domain::integer(7, &mut flow, Span::default()).unwrap(),
            origin: Some(Origin::Expression(0, 1)),
        }
    );
    let c = frame(Some(&second), &mut flow);
    let joined = partial
        .join(&c, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let cell = joined.slot(0).unwrap();
    assert!(cell.control);
    let expected = Domain::integer(7, &mut flow, Span::default())
        .unwrap()
        .join(
            &Domain::integer(9, &mut flow, Span::default()).unwrap(),
            &mut flow,
            Span::default(),
        )
        .unwrap();
    assert_eq!(
        cell.data,
        Data::Conditional {
            domain: expected.clone(),
            origin: Some(Origin::Expression(0, 1))
        }
    );
    let other = frame(Some(&value(9, 2, &mut flow)), &mut flow);
    let joined = joined
        .join(&other, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(
        joined.slot(0).unwrap().data,
        Data::Conditional {
            domain: expected,
            origin: None
        }
    );
    assert_eq!(a.slot(0), Some(&first));
    assert_eq!(b.slot(0), Some(&empty));
}

#[test]
pub(crate) fn scalar_slot_joins_preserve_missing_and_opaque_evidence_associatively() {
    let mut flow = Flow::new();
    let mut cells = [
        Cell {
            data: Data::Unavailable,
            control: false,
        },
        Cell {
            data: Data::Uninitialized,
            control: false,
        },
        Cell {
            data: Data::Partial,
            control: true,
        },
        Cell {
            data: Data::Initialized {
                version: None,
                domain: None,
                origin: None,
            },
            control: false,
        },
        value(7, 1, &mut flow),
        value(9, 2, &mut flow),
        Cell {
            data: Data::Conditional {
                domain: Domain::integer(7, &mut flow, Span::default()).unwrap(),
                origin: Some(Origin::Expression(0, 1)),
            },
            control: true,
        },
    ];
    cells[5].control = true;
    let mut states = cells
        .iter()
        .map(|cell| frame(Some(cell), &mut flow))
        .collect::<Vec<_>>();
    states.push(frame(None, &mut flow));
    states.push(State::new(0, FALSE, &mut flow, Span::default()).unwrap());
    for a in &states {
        assert_eq!(
            a.join(a, &mut flow, Span::default(), MAX_EDGES).unwrap(),
            *a
        );
        for b in &states {
            let ab = a.join(b, &mut flow, Span::default(), MAX_EDGES).unwrap();
            assert_eq!(
                ab,
                b.join(a, &mut flow, Span::default(), MAX_EDGES).unwrap()
            );
            for c in &states {
                let bc = b.join(c, &mut flow, Span::default(), MAX_EDGES).unwrap();
                assert_eq!(
                    ab.join(c, &mut flow, Span::default(), MAX_EDGES).unwrap(),
                    a.join(&bc, &mut flow, Span::default(), MAX_EDGES).unwrap()
                );
            }
        }
    }
    for (index, expected) in [
        (0, Data::Unavailable),
        (2, Data::Partial),
        (3, Data::Partial),
        (7, Data::Unavailable),
    ] {
        let joined = states[6]
            .join(&states[index], &mut flow, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(joined.slot(0).unwrap().data, expected);
    }
}
