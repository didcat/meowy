use super::super::capture::tests::capture;
use super::*;

pub(super) fn setup(source: &str) -> (Checker, Snapshots, definitions::Definitions, PointId) {
    let (mut checker, program, reports, snapshots) = capture(source);
    let (versions, events, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let (definitions, _) = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    let point = *definitions.storage.keys().min().unwrap();
    (checker, snapshots, definitions, point)
}

#[test]
pub(crate) fn scalar_bind_transfer_preserves_before_state_and_uses_observed_rhs_values() {
    for source in ["x:7", "f:(n<uint8>){copy<uint8>:((n)~<uint8>)}"] {
        let (mut checker, mut snapshots, definitions, point) = setup(source);
        let storage = &definitions.storage[&point];
        let before = snapshots.rows[&Port::Operation(point)].before;
        let old = format!("{:?}{:?}", snapshots.rows, snapshots.expressions);
        let Some(Value::Known(after)) = snapshots
            .storage(
                point,
                before,
                &definitions,
                &mut checker.flow,
                Span::default(),
            )
            .unwrap()
        else {
            panic!()
        };
        let Data::Initialized {
            version,
            domain,
            origin,
        } = &snapshots.states[after]
            .get(storage.version.local)
            .unwrap()
            .data
        else {
            panic!()
        };
        let value = snapshots.expressions[&storage.input]
            .value
            .as_ref()
            .unwrap();
        assert_eq!(*version, Some(storage.version));
        assert_eq!(domain.as_ref(), Some(&value.domain));
        assert_eq!(*origin, value.origin);
        let Value::Known(before) = before else {
            panic!()
        };
        assert_eq!(
            snapshots.states[before]
                .get(storage.version.local)
                .unwrap()
                .data,
            Data::Uninitialized
        );
        assert_eq!(
            format!("{:?}{:?}", snapshots.rows, snapshots.expressions),
            old
        );
    }
}

#[test]
pub(crate) fn scalar_bind_transfer_keeps_missing_state_separate_from_unknown_value() {
    let (mut checker, mut snapshots, definitions, point) = setup("x:7");
    let storage = &definitions.storage[&point];
    let before = snapshots.rows[&Port::Operation(point)].before;
    snapshots.expressions.remove(&storage.input);
    assert_eq!(
        snapshots
            .storage(
                point,
                Value::Missing,
                &definitions,
                &mut checker.flow,
                Span::default()
            )
            .unwrap(),
        Some(Value::Missing)
    );
    let Some(Value::Known(after)) = snapshots
        .storage(
            point,
            before,
            &definitions,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap()
    else {
        panic!()
    };
    let cell = snapshots.states[after].get(storage.version.local).unwrap();
    assert!(cell.control);
    assert_eq!(
        cell.data,
        Data::Initialized {
            version: Some(storage.version),
            domain: Some(storage.declared.clone()),
            origin: Some(Origin::Version(storage.version))
        }
    );
}

#[test]
pub(crate) fn scalar_bind_transfer_bounds_allocations_and_rejects_stale_sources_atomically() {
    let (mut checker, mut snapshots, mut definitions, point) = setup("x:7");
    let before = snapshots.rows[&Port::Operation(point)].before;
    let room = snapshots.room();
    let start = checker.flow.work;
    let expected = snapshots
        .storage(
            point,
            before,
            &definitions,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    let used = room - snapshots.room();
    let cost = checker.flow.work - start;
    let Some(Value::Known(expected)) = expected else {
        panic!()
    };
    let expected = snapshots.states[expected]
        .snapshot(&mut checker.flow, Span::default(), MAX_EDGES)
        .unwrap();
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        snapshots.parts = room;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let len = snapshots.states.len();
        let result = snapshots.storage(
            point,
            before,
            &definitions,
            &mut checker.flow,
            Span::default(),
        );
        assert_eq!(result.is_ok(), pass);
        assert_eq!(snapshots.states.len(), len + usize::from(pass));
        if let Ok(Some(Value::Known(id))) = result {
            assert_eq!(snapshots.states[id], expected);
            assert_eq!(snapshots.room(), 0);
        }
    }
    checker.flow.work = 0;
    checker.flow.full = false;
    snapshots.parts = MAX_EDGES;
    definitions.storage.get_mut(&point).unwrap().input = usize::MAX;
    assert_eq!(
        snapshots
            .storage(
                point,
                before,
                &definitions,
                &mut checker.flow,
                Span::default()
            )
            .unwrap(),
        Some(Value::Missing)
    );
    definitions.storage.get_mut(&point).unwrap().version.owner += 1;
    assert!(
        snapshots
            .storage(
                point,
                before,
                &definitions,
                &mut checker.flow,
                Span::default()
            )
            .is_err()
    );
}
