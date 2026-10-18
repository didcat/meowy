use super::super::super::domain::Values;
use super::super::tests::value;
use super::tests::frame;
use super::*;

pub(super) fn data(domain: Domain) -> Cell {
    Cell {
        data: Data::Initialized {
            version: None,
            domain: Some(domain),
            origin: None,
        },
        control: false,
    }
}

#[test]
pub(crate) fn scalar_state_boundaries_keep_boolean_null_and_integer_join_canonicalization() {
    let mut flow = Flow::new();
    let yes = frame(&data(Domain::boolean(true)), &mut flow);
    let null = frame(&data(Domain::NULL), &mut flow);
    let joined = yes
        .join(&null, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(
        joined.get(0).unwrap().data,
        data(Domain {
            values: Some(Values::Bool(2)),
            null: true
        })
        .data
    );
    let mut finite = Domain::integer(-128, &mut flow, Span::default()).unwrap();
    for n in -127..-112 {
        finite = finite
            .join(
                &Domain::integer(n, &mut flow, Span::default()).unwrap(),
                &mut flow,
                Span::default(),
            )
            .unwrap();
    }
    let left = frame(&data(finite), &mut flow);
    let right = frame(
        &data(Domain::integer(127, &mut flow, Span::default()).unwrap()),
        &mut flow,
    );
    let joined = left
        .join(&right, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let full = Domain::declared(
        &hir::Type::Int {
            bits: 8,
            signed: true,
        },
        &mut flow,
        Span::default(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(joined.get(0).unwrap().data, data(full.clone()).data);
    let seven = frame(
        &data(Domain::integer(7, &mut flow, Span::default()).unwrap()),
        &mut flow,
    );
    let unknown = frame(&data(full.clone()), &mut flow);
    let joined = seven
        .join(&unknown, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(joined.get(0).unwrap().data, data(full).data);
}

#[test]
pub(crate) fn scalar_state_boundaries_join_associatively_without_losing_control_or_missing_data() {
    let mut flow = Flow::new();
    let mut marked = value(0, 2, 9, &mut flow);
    marked.control = true;
    let states = [
        State::new(0, TRUE, &mut flow, Span::default()).unwrap(),
        frame(
            &Cell {
                data: Data::Unavailable,
                control: false,
            },
            &mut flow,
        ),
        frame(
            &Cell {
                data: Data::Uninitialized,
                control: false,
            },
            &mut flow,
        ),
        frame(
            &Cell {
                data: Data::Partial,
                control: true,
            },
            &mut flow,
        ),
        frame(
            &Cell {
                data: Data::Initialized {
                    version: None,
                    domain: None,
                    origin: None,
                },
                control: false,
            },
            &mut flow,
        ),
        frame(&value(0, 1, 7, &mut flow), &mut flow),
        frame(&marked, &mut flow),
        State::new(0, FALSE, &mut flow, Span::default()).unwrap(),
    ];
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
}

#[test]
pub(crate) fn scalar_state_boundaries_reject_guard_exhaustion_without_accepting_fallback_true() {
    let mut flow = Flow::new();
    let a = flow.fresh();
    let b = flow.fresh();
    let state = State::new(0, a, &mut flow, Span::default()).unwrap();
    let before = format!("{state:?}");
    while flow.nodes.len() < crate::flow::MAX_NODES {
        flow.fresh();
    }
    assert!(!flow.exceeded());
    assert!(
        state
            .restrict(b, &mut flow, Span::default(), MAX_EDGES)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(flow.exceeded());
    assert_eq!(format!("{state:?}"), before);
}

#[test]
pub(crate) fn scalar_state_boundaries_validate_cells_before_copying_or_discarding_them() {
    let mut flow = Flow::new();
    let mut state = frame(&value(0, 1, 7, &mut flow), &mut flow);
    let Data::Initialized { origin, .. } = &mut state.cells.get_mut(&0).unwrap().data else {
        panic!()
    };
    *origin = Some(Origin::Expression(1, 1));
    let before = format!("{state:?}");
    let bottom = State::new(0, FALSE, &mut flow, Span::default()).unwrap();
    assert!(
        state
            .snapshot(&mut flow, Span::default(), MAX_EDGES)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(
        state
            .restrict(FALSE, &mut flow, Span::default(), MAX_EDGES)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(
        bottom
            .join(&state, &mut flow, Span::default(), MAX_EDGES)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{state:?}"), before);
}
