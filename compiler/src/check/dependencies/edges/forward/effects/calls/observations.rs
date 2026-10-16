use super::{super::tests::checked, *};

#[test]
pub(crate) fn call_observations_keep_operation_and_result_visits_independent() {
    let source = "f<int32>:(a<int32>,b<int32>){->a+b};alias:f;x:f(1,2);y:3.(alias,4);recur<int32>:(n<int32>){->recur(n)}";
    let (mut checker, mut reports) = checked(source, true);
    let expected = reports.effects.clone();
    let calls: Vec<_> = checker
        .invocations
        .values()
        .map(|call| (call.point, call.owner))
        .collect();
    assert_eq!(calls.len(), 3);
    assert!(calls.iter().any(|(_, owner)| *owner != 0));
    for (id, owner) in calls {
        for ports in [
            vec![],
            vec![Port::Entry(id)],
            vec![Port::Normal(id)],
            vec![Port::Operation(id)],
            vec![Port::Normal(id), Port::Operation(id)],
            vec![Port::Operation(id), Port::Normal(id), Port::Operation(id)],
        ] {
            let operation = ports.contains(&Port::Operation(id));
            let result = ports.contains(&Port::Normal(id));
            let observed = operation || result;
            for (_, walk) in reports.entries.values_mut() {
                walk.ports.clear();
            }
            reports.entries.get_mut(&owner).unwrap().1.ports = ports;
            let effects = checker
                .operation_effects(&reports, Span::default())
                .unwrap();
            assert_eq!(effects.len(), usize::from(observed));
            let mut expected_call = expected[&id].clone();
            let Effect::Call {
                operation: op,
                result: end,
                ..
            } = &mut expected_call.1
            else {
                panic!()
            };
            *op = operation;
            *end = result;
            assert_eq!(effects.get(&id), observed.then_some(&expected_call));
            assert_eq!(reports.effects, expected);
        }
    }
}

#[test]
pub(crate) fn call_observations_keep_conditional_returns_and_independent_owners() {
    let source = "flag:false;f<int32>:(a<int32>,b<int32>){->a+b};alias:f;|flag|x:1.(alias,2);recur<int32>:(n<int32>){->recur(n)};stop<never>:(){'loop{'loop.restart()}};stop();tail:f(3,4)";
    let (checker, reports) = checked(source, true);
    let mut observed = 0;
    for call in checker.invocations.values() {
        let walk = &reports.entries[&call.owner].1;
        if !walk.ports.contains(&Port::Operation(call.point)) {
            assert_eq!(&source[call.span.start..call.span.end], "f(3,4)");
            assert!(!walk.ports.contains(&Port::Normal(call.point)));
            continue;
        }
        observed += 1;
        assert_eq!(
            walk.ports.contains(&Port::Normal(call.point)),
            call.may_return
        );
        let operation = walk
            .ports
            .iter()
            .position(|port| *port == Port::Operation(call.point))
            .unwrap();
        let mut previous = 0;
        for arg in &call.args {
            let entry = walk
                .ports
                .iter()
                .position(|port| *port == Port::Entry(*arg))
                .unwrap();
            let normal = walk
                .ports
                .iter()
                .position(|port| *port == Port::Normal(*arg))
                .unwrap();
            assert!(previous < entry && entry < normal && normal < operation);
            previous = normal;
        }
        assert!(walk.ports.iter().all(|port| match port {
            Port::BlockEntry(body) => checker.bodies[body].owner == call.owner,
            _ => true,
        }));
    }
    assert_eq!(observed, 3);
}

#[test]
pub(crate) fn call_observations_reject_nonreturning_results_and_foreign_owners() {
    for fault in 0..3 {
        let (mut checker, mut reports) =
            checked("stop<never>:(){'loop{'loop.restart()}};stop()", false);
        let call = checker.invocations.first_key_value().unwrap().1;
        let (id, owner) = (call.point, call.owner);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.clear();
        }
        match fault {
            0 => reports
                .entries
                .get_mut(&owner)
                .unwrap()
                .1
                .ports
                .push(Port::Normal(id)),
            1 => reports
                .entries
                .get_mut(&(owner + 1))
                .unwrap()
                .1
                .ports
                .push(Port::Operation(id)),
            2 => {
                reports
                    .entries
                    .get_mut(&owner)
                    .unwrap()
                    .1
                    .ports
                    .push(Port::Operation(id));
                reports.index.operations.remove(&id);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .operation_effects(&reports, Span::default())
            .unwrap_err();
        assert!(error.message.contains("call-effect identity"), "{error:?}");
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn call_result_observations_do_not_inspect_callee_completion() {
    let source = "d:@\"debug\";fail<int32>:(){d.panic(\"stop\")};sum<int32>:(a<int32>,b<int32>){->a+b};x:sum(fail(),2)";
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.invocations.len(), 2);
    for call in checker.invocations.values() {
        assert!(matches!(
            reports.effects[&call.point].1,
            Effect::Call {
                may_return: true,
                operation: true,
                result: true,
                ..
            }
        ));
        assert!(reports.calls.sites.contains_key(&call.site));
        if call.args.is_empty() {
            let (body, walk) = &reports.entries[&(call.function + 1)];
            assert!(!walk.ports.contains(&Port::BlockNormal(*body)));
            assert!(!walk.ports.contains(&Port::BlockResult(*body)));
        } else {
            assert!(
                reports.entries[&call.owner]
                    .1
                    .ports
                    .contains(&Port::Normal(call.args[1]))
            );
        }
    }
}

#[test]
pub(crate) fn call_observations_stop_before_later_arguments_and_outer_operation() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};f<int32>:(a<int32>,b<int32>){->a+b};x:f(stop(),{d.print(\"later\");->2})";
    let (checker, reports) = checked(source, false);
    let outer = checker
        .invocations
        .values()
        .find(|call| call.args.len() == 2)
        .unwrap();
    let stop = checker
        .invocations
        .values()
        .find(|call| call.args.is_empty())
        .unwrap();
    let ports = &reports.entries[&0].1.ports;
    assert!(ports.contains(&Port::Entry(outer.point)));
    assert!(ports.contains(&Port::Operation(stop.point)));
    for port in [
        Port::Normal(stop.point),
        Port::Entry(outer.args[1]),
        Port::Operation(outer.point),
        Port::Normal(outer.point),
    ] {
        assert!(!ports.contains(&port));
    }
    assert!(reports.effects.contains_key(&stop.point));
    assert!(!reports.effects.contains_key(&outer.point));
}
