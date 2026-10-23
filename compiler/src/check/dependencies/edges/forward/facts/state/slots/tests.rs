use super::*;

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
        Data::Partial
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
