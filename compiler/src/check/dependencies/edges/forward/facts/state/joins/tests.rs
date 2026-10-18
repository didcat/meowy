use super::super::tests::value;
use super::*;
use versions::Definition;

pub(super) fn frame(cell: &Cell, flow: &mut Flow) -> State {
    let mut state = State::new(0, TRUE, flow, Span::default()).unwrap();
    state
        .put(0, cell, flow, Span::default(), MAX_EDGES)
        .unwrap();
    state
}

#[test]
pub(crate) fn scalar_state_joins_ignore_dead_inputs_without_guard_based_value_refinement() {
    let mut flow = Flow::new();
    let guard = flow.fresh();
    let left = frame(&value(0, 1, 7, &mut flow), &mut flow)
        .restrict(guard, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let right = frame(&value(0, 2, 9, &mut flow), &mut flow);
    let dead = right
        .restrict(FALSE, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(
        left.join(&dead, &mut flow, Span::default(), MAX_EDGES)
            .unwrap(),
        left
    );
    let right = right
        .restrict(flow.not(guard), &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let joined = left
        .join(&right, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(joined.reach, TRUE);
    let Data::Initialized {
        version,
        domain,
        origin,
    } = &joined.get(0).unwrap().data
    else {
        panic!()
    };
    assert!(version.is_none() && origin.is_none());
    let expected = Domain::integer(7, &mut flow, Span::default())
        .unwrap()
        .join(
            &Domain::integer(9, &mut flow, Span::default()).unwrap(),
            &mut flow,
            Span::default(),
        )
        .unwrap();
    assert_eq!(domain.as_ref(), Some(&expected));
    let restricted = joined
        .restrict(guard, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(restricted.get(0), joined.get(0));
}

#[test]
pub(crate) fn scalar_state_joins_keep_version_origin_and_control_agreement_separate() {
    let mut flow = Flow::new();
    let original = value(0, 1, 7, &mut flow);
    let left = frame(&original, &mut flow);
    assert_eq!(
        left.join(&left, &mut flow, Span::default(), MAX_EDGES)
            .unwrap(),
        left
    );
    let mut copy = original.clone();
    copy.control = true;
    let Data::Initialized { version, .. } = &mut copy.data else {
        panic!()
    };
    *version = Some(Version {
        owner: 0,
        local: 0,
        definition: Definition::Write(2),
    });
    let right = frame(&copy, &mut flow);
    let joined = left
        .join(&right, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(joined.get(0).unwrap().control);
    assert!(matches!(
        joined.get(0).unwrap().data,
        Data::Initialized {
            version: None,
            origin: Some(Origin::Expression(0, 1)),
            ..
        }
    ));
    let equal = frame(&value(0, 3, 7, &mut flow), &mut flow);
    let joined = joined
        .join(&equal, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(matches!(
        joined.get(0).unwrap().data,
        Data::Initialized { origin: None, .. }
    ));
    assert!(joined.get(0).unwrap().control);
}

#[test]
pub(crate) fn scalar_state_joins_keep_live_missing_and_partial_initialization_conservative() {
    let mut flow = Flow::new();
    let known = frame(&value(0, 1, 7, &mut flow), &mut flow);
    let missing = State::new(0, TRUE, &mut flow, Span::default()).unwrap();
    let joined = known
        .join(&missing, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(joined.get(0).unwrap().data, Data::Unavailable);
    let uninit = frame(
        &Cell {
            data: Data::Uninitialized,
            control: false,
        },
        &mut flow,
    );
    let joined = known
        .join(&uninit, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(joined.get(0).unwrap().data, Data::Partial);
    let opaque = frame(
        &Cell {
            data: Data::Initialized {
                version: None,
                domain: None,
                origin: None,
            },
            control: false,
        },
        &mut flow,
    );
    let joined = known
        .join(&opaque, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(
        joined.get(0).unwrap().data,
        Data::Initialized {
            version: None,
            domain: None,
            origin: None
        }
    );
}

#[test]
pub(crate) fn scalar_state_joins_bound_work_payload_and_reject_invalid_guards_and_owners() {
    let mut flow = Flow::new();
    let left = frame(&value(0, 1, 7, &mut flow), &mut flow);
    let right = frame(&value(0, 2, 9, &mut flow), &mut flow);
    let before = format!("{left:?}{right:?}");
    let start = flow.work;
    let joined = left
        .join(&right, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = flow.work - start;
    for (limit, extra, pass) in [
        (joined.weight() - 1, 0, false),
        (joined.weight(), 1, false),
        (joined.weight(), 0, true),
    ] {
        flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        flow.full = false;
        assert_eq!(
            left.join(&right, &mut flow, Span::default(), limit).is_ok(),
            pass
        );
        assert_eq!(format!("{left:?}{right:?}"), before);
    }
    flow.work = 0;
    flow.full = false;
    assert!(
        left.restrict(usize::MAX, &mut flow, Span::default(), MAX_EDGES)
            .unwrap_err()
            .message
            .contains("identity")
    );
    let foreign = State::new(1, FALSE, &mut flow, Span::default()).unwrap();
    assert!(
        left.join(&foreign, &mut flow, Span::default(), MAX_EDGES)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(left.restrict(FALSE, &mut flow, Span::default(), 0).is_err());
}
