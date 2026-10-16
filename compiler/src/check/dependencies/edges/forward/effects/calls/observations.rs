use super::{super::tests::checked, *};

#[test]
pub(crate) fn call_observations_currently_require_operation_visits() {
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
            let observed = ports.contains(&Port::Operation(id));
            for (_, walk) in reports.entries.values_mut() {
                walk.ports.clear();
            }
            reports.entries.get_mut(&owner).unwrap().1.ports = ports;
            let effects = checker
                .operation_effects(&reports, Span::default())
                .unwrap();
            assert_eq!(effects.len(), usize::from(observed));
            assert_eq!(effects.get(&id), observed.then(|| &expected[&id]));
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
