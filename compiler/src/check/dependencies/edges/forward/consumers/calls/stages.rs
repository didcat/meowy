use super::*;

#[test]
pub(crate) fn call_argument_links_require_their_own_projection_and_source_observations() {
    let (mut checker, mut reports) =
        checked("f:(a<int32><null>,b<int32>){};r:7.{->$;->tag:true};s:{->9;->tag:false};r.(f,s)");
    let call = checker.invocations.first_key_value().unwrap().1;
    let (point, owner) = (call.point, call.owner);
    let id = projection(&checker, call.args[0]);
    let next = projection(&checker, call.args[1]);
    let port = Port::Projection { point: id, step: 0 };
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 2);
    assert_ne!(
        expected[&port].1.block,
        expected[&Port::Projection {
            point: next,
            step: 0
        }]
            .1
            .block
    );
    let dispatch = reports.results[&expected[&port].1.block]
        .1
        .dispatch
        .unwrap();
    let call = reports.effects[&point].clone();
    for call_state in 0..4 {
        reports.effects.remove(&point);
        if call_state != 0 {
            let mut call = call.clone();
            let Effect::Call {
                operation, result, ..
            } = &mut call.1
            else {
                panic!()
            };
            *operation = call_state & 1 != 0;
            *result = call_state & 2 != 0;
            reports.effects.insert(point, call);
        }
        for published in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.result = published;
            for (projected, operation, result) in [
                (true, false, false),
                (true, true, true),
                (false, true, false),
                (false, false, true),
            ] {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.projected = projected;
                op.operation = operation;
                op.result = result;
                if operation || result {
                    reports.index.operations.insert(id, owner);
                } else {
                    reports.index.operations.remove(&id);
                }
                let mut wanted = expected.clone();
                if !projected || !published {
                    wanted.remove(&port);
                }
                let before = format!("{reports:?}{:?}", checker.edge_counts());
                assert_eq!(
                    checker.slot_uses(&reports, Span::default()).unwrap(),
                    wanted
                );
                assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
            }
        }
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn call_argument_links_keep_earlier_projections_before_stopped_later_inputs() {
    let source = "d:@\"debug\";f:(a<int32>,b<int32>,c<int32>){};r:{->7;->tag:true};f(((r)),d.panic(\"stop\"),r)";
    let (checker, reports) = checked(source);
    let call = checker.invocations.first_key_value().unwrap().1;
    let first = projection(&checker, call.args[0]);
    let last = projection(&checker, call.args[2]);
    let ports = &reports.entries[&call.owner].1.ports;
    let port = Port::Projection {
        point: first,
        step: 0,
    };
    assert!(reports.slot_uses.contains_key(&port));
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(ports.contains(&port));
    assert!(ports.contains(&Port::Normal(call.args[0])));
    assert!(ports.contains(&Port::Entry(call.args[1])));
    assert_eq!(checker.coercions[&call.args[1]].kind, CoercionKind::Stopped);
    assert!(!reports.effects.contains_key(&call.point));
    assert!(!reports.effects.contains_key(&last));
    assert!(!reports.slot_uses.contains_key(&Port::Projection {
        point: last,
        step: 0
    }));
    for port in [
        Port::Normal(call.args[1]),
        Port::Entry(call.args[2]),
        Port::Operation(call.point),
        Port::Normal(call.point),
    ] {
        assert!(!ports.contains(&port));
    }
}

#[test]
pub(crate) fn call_argument_links_keep_parameters_call_results_and_reference_loads_opaque() {
    for source in [
        "f:(a<int32>){};outer:(r<{-><int32>;tag<boolean>}>){f(r)}",
        "f:(a<int32>){};r:={->7;->tag:true};f(r)",
        "f:(a<int32>){};r:{->7;->tag:true};p:&r;f(*p)",
        "f:(a<int32>){};make<{-><int32>;tag<boolean>}>:(){->7;->tag:true};f(make())",
        "f:(a<int32>){};outer:(r<{-><never>;tag<boolean>}>){f(r)}",
    ] {
        let (checker, reports) = checked(source);
        let call = checker
            .invocations
            .values()
            .find(|call| call.args.len() == 1)
            .unwrap();
        let id = projection(&checker, call.args[0]);
        let (_, Effect::Coercion(op)) = &reports.effects[&id] else {
            panic!()
        };
        assert!(op.primary && op.projected);
        assert_eq!(op.result, op.op != CoercionKind::Stopped);
        assert!(reports.slot_uses.is_empty(), "{source}");
        if op.op == CoercionKind::Stopped {
            assert!(!reports.effects.contains_key(&call.point));
        }
    }
}
