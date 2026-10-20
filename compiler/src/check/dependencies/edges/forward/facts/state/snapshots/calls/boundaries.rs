use super::super::capture::tests::capture;
use super::*;
use domain::Values;

pub(super) fn setup() -> (
    Checker,
    Snapshots,
    definitions::Definitions,
    Mutation,
    Value,
) {
    let (mut checker, program, reports, snapshots) =
        capture("f:(a<&!boolean>,b<&!boolean>){};x:=false;y:=true;f(&!x,&!y)");
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
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    let calls = checker
        .qualify_borrow_calls(&program, &reports, &facts, Span::default())
        .unwrap();
    let (&point, call) = calls.first_key_value().unwrap();
    let before = snapshots.rows[&Port::Operation(point)].before;
    (checker, snapshots, definitions, call.clone(), before)
}

#[test]
pub(crate) fn scalar_call_boundaries_preserve_initialization_and_monotone_control() {
    for data in [
        Data::Unavailable,
        Data::Uninitialized,
        Data::Partial,
        Data::Initialized {
            version: None,
            domain: None,
            origin: None,
        },
    ] {
        let (mut checker, mut snapshots, definitions, call, before) = setup();
        let Value::Known(id) = before else { panic!() };
        let local = call.roots[0];
        let mut state = snapshots.states[id]
            .snapshot(&mut checker.flow, Span::default(), MAX_EDGES)
            .unwrap();
        state
            .put(
                local,
                &Cell {
                    data: data.clone(),
                    control: true,
                },
                &mut checker.flow,
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        let before = snapshots.keep(state, Span::default()).unwrap();
        let input = snapshots.rows.get_mut(&call.input).unwrap();
        input.before = before;
        input.after = before;
        let Value::Known(after) = snapshots
            .call(
                before,
                &call,
                &definitions,
                &mut checker.flow,
                Span::default(),
            )
            .unwrap()
        else {
            panic!()
        };
        let cell = snapshots.states[after].get(local).unwrap();
        assert!(cell.control);
        let expected = if matches!(data, Data::Initialized { .. }) {
            Data::Initialized {
                version: None,
                domain: Some(Domain {
                    values: Some(Values::Bool(3)),
                    null: false,
                }),
                origin: None,
            }
        } else {
            data
        };
        assert_eq!(cell.data, expected);
        assert!(matches!(
            snapshots.states[id].get(local).unwrap().data,
            Data::Initialized {
                version: Some(_),
                ..
            }
        ));
    }
    let (mut checker, mut snapshots, definitions, mut call, before) = setup();
    call.control = true;
    let Value::Known(after) = snapshots
        .call(
            before,
            &call,
            &definitions,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap()
    else {
        panic!()
    };
    assert!(
        call.roots
            .iter()
            .all(|local| snapshots.states[after].get(*local).unwrap().control)
    );
}

#[test]
pub(crate) fn scalar_call_boundaries_publish_no_partial_state_for_unknown_or_stale_inputs() {
    for fault in 0..7 {
        let (mut checker, mut snapshots, mut definitions, mut call, before) = setup();
        let second = call.roots[1];
        match fault {
            0 => call.unknown = true,
            1 => {
                definitions.declarations.remove(&second);
            }
            2 => call.owner += 1,
            3 => snapshots.rows.get_mut(&call.input).unwrap().after = Value::Missing,
            4 => {
                definitions
                    .declarations
                    .get_mut(&second)
                    .unwrap()
                    .version
                    .owner += 1
            }
            5 => {
                let Port::Operation(point) = definitions.declarations[&second].port else {
                    panic!()
                };
                definitions.storage.get_mut(&point).unwrap().declared = Domain::NULL;
            }
            6 => call.roots.push(call.roots[0]),
            _ => unreachable!(),
        }
        let prior = format!("{snapshots:?}");
        let result = snapshots.call(
            before,
            &call,
            &definitions,
            &mut checker.flow,
            Span::default(),
        );
        if fault < 2 {
            assert_eq!(result.unwrap(), Value::Missing);
        } else {
            assert!(result.unwrap_err().message.contains("identity"), "{fault}");
        }
        assert_eq!(format!("{snapshots:?}"), prior);
    }
}

#[test]
pub(crate) fn scalar_call_boundaries_reserve_exact_snapshot_payload_and_work() {
    let (mut checker, mut snapshots, definitions, call, before) = setup();
    let room = snapshots.room();
    let start = checker.flow.work;
    let Value::Known(after) = snapshots
        .call(
            before,
            &call,
            &definitions,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap()
    else {
        panic!()
    };
    let cost = checker.flow.work - start;
    let used = room - snapshots.room();
    let expected = snapshots.states[after]
        .snapshot(&mut checker.flow, Span::default(), MAX_EDGES)
        .unwrap();
    let history = format!("{:?}{:?}", snapshots.rows, snapshots.expressions);
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        snapshots.parts = room;
        let len = snapshots.states.len();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = snapshots.call(
            before,
            &call,
            &definitions,
            &mut checker.flow,
            Span::default(),
        );
        assert_eq!(result.is_ok(), pass);
        if let Ok(Value::Known(after)) = result {
            assert_eq!(snapshots.states[after], expected);
        }
        assert_eq!(snapshots.states.len(), len + usize::from(pass));
        assert_eq!(snapshots.room(), if pass { 0 } else { room });
        assert_eq!(
            format!("{:?}{:?}", snapshots.rows, snapshots.expressions),
            history
        );
    }
}
