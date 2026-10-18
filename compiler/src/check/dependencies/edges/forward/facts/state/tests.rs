use super::*;
use crate::flow::TRUE;
use versions::Definition;

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
            .snapshot(&mut flow, Span::default(), cost - 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(
        state,
        state.snapshot(&mut flow, Span::default(), cost).unwrap()
    );
}
