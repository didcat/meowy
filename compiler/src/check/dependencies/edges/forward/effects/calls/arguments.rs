use super::{super::tests::checked, *};

#[test]
pub(crate) fn argument_ports_keep_wrapped_receiver_order_and_empty_calls() {
    let source = "f:(a<int32>,unused<boolean>,b<int32>){};r:{->7;->tag:true};((r)).(f,true,9);empty:(){};empty()";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.invocations.len(), 2);
    for call in checker.invocations.values() {
        let ports = &reports.entries[&call.owner].1.ports;
        let params = &reports.signatures.functions[&call.function].params;
        assert_eq!(params.len(), call.args.len());
        for arg in &call.args {
            assert_eq!(checker.points[*arg].owner, call.owner);
            assert!(ports.contains(&Port::Entry(*arg)));
            assert!(ports.contains(&Port::Normal(*arg)));
        }
        if !call.args.is_empty() {
            let texts = call
                .args
                .iter()
                .map(|arg| {
                    let span = checker.points[*arg].span;
                    &source[span.start..span.end]
                })
                .collect::<Vec<_>>();
            assert_eq!(texts, ["((r))", "true", "9"]);
        }
    }
}

#[test]
pub(crate) fn argument_ports_keep_stopped_prefixes_without_outer_call_observations() {
    for (args, stopped) in [("d.panic(\"stop\"),2,3", 0), ("1,d.panic(\"stop\"),3", 1)] {
        let source = format!("d:@\"debug\";f:(a<int32>,b<int32>,c<int32>){{}};f({args})");
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source, false);
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let pairs = checker
            .checked_call_inputs(&reports, call.site, Span::default())
            .unwrap()
            .pairs
            .collect::<Vec<_>>();
        let ports = &reports.entries[&call.owner].1.ports;
        assert_eq!(pairs.len(), 3);
        for (position, (_, arg)) in pairs.into_iter().enumerate() {
            assert_eq!(ports.contains(&Port::Entry(arg)), position <= stopped);
            assert_eq!(ports.contains(&Port::Normal(arg)), position < stopped);
        }
        assert!(!reports.effects.contains_key(&call.point));
        assert!(!reports.calls.sites.contains_key(&call.site));
    }
}

#[test]
pub(crate) fn argument_ports_remain_independent_of_call_and_callee_completion() {
    let source =
        "d:@\"debug\";fail<int32>:(){d.panic(\"stop\")};f:(a<int32>,b<int32>){};f(fail(),2)";
    let (mut checker, mut reports) = checked(source, false);
    let call = checker
        .invocations
        .values()
        .find(|call| call.args.len() == 2)
        .unwrap()
        .clone();
    for arg in &call.args {
        assert!(
            reports.entries[&call.owner]
                .1
                .ports
                .contains(&Port::Normal(*arg))
        );
    }
    for (ports, expected) in [
        (vec![], [(false, false); 2]),
        (vec![Port::Operation(call.point)], [(false, false); 2]),
        (vec![Port::Normal(call.point)], [(false, false); 2]),
        (
            vec![Port::Entry(call.args[0])],
            [(true, false), (false, false)],
        ),
        (
            vec![Port::Normal(call.args[0])],
            [(false, true), (false, false)],
        ),
        (
            vec![
                Port::Entry(call.args[0]),
                Port::Entry(call.args[0]),
                Port::Normal(call.args[1]),
            ],
            [(true, false), (false, true)],
        ),
    ] {
        reports.entries.get_mut(&call.owner).unwrap().1.ports = ports.clone();
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let pairs = checker
            .checked_call_inputs(&reports, call.site, Span::default())
            .unwrap()
            .pairs
            .collect::<Vec<_>>();
        let observed = pairs
            .iter()
            .map(|(_, arg)| {
                (
                    ports.contains(&Port::Entry(*arg)),
                    ports.contains(&Port::Normal(*arg)),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(observed, expected);
        assert_eq!(pairs.len(), 2);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
