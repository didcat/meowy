use super::{super::tests::checked, *};

#[test]
pub(crate) fn grouped_consumers_link_owned_fields_and_observed_primary_ports() {
    for source in [
        "v:(({->n:1})).n",
        "v:-(({->1;->tag:true}))",
        "v:!(({->true;->tag:false}))",
        "v:(({->1;->tag:true}))+1",
    ] {
        let (mut checker, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), 1, "{source}");
        let (&port, (owner, slot)) = reports.slot_uses.first_key_value().unwrap();
        let input = match port {
            Port::Operation(id) => checker.fields[&id].input,
            Port::Projection { point, step } => match &reports.effects[&point].1 {
                Effect::Unary(op) => op.input,
                Effect::Binary(op) => op.inputs[step],
                _ => panic!(),
            },
            _ => panic!(),
        };
        assert!(!reports.consumers.contains_key(&input));
        let anchor = checker
            .grouped_consumer(&reports, input, *owner, Span::default())
            .unwrap()
            .unwrap();
        assert_eq!(reports.consumers[&anchor], (*owner, slot.block));
        assert_eq!(reports.results[&slot.block].1.consumer, Some(anchor));
    }
}

#[test]
pub(crate) fn grouped_consumers_keep_opaque_terminals_and_stopped_stages() {
    for source in [
        "r:{->n:1};p:&r;v:((p)).n",
        "f<{n<int32>}>:(){->n:1};v:((f())).n",
        "x:((false&&true));y:((false||true))",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let source = "d:@\"debug\";v:(({->1;->tag:true}))+{d.panic(\"stop\")}";
    let (_, reports) = checked(source);
    let (&port, _) = reports.slot_uses.first_key_value().unwrap();
    let Port::Projection { point, step: 0 } = port else {
        panic!()
    };
    let Effect::Binary(op) = &reports.effects[&point].1 else {
        panic!()
    };
    assert_eq!(op.projected, [true, false]);
    assert!(!op.operation && !op.result);
}

#[test]
pub(crate) fn grouped_consumers_keep_composed_results_and_function_owners() {
    let source = "f<int32>:(){->(({->n:1})).n};v:(({->(({->x:1}));->y:2})).y";
    let (checker, reports) = checked(source);
    assert_eq!(reports.slot_uses.len(), 4);
    assert_eq!(
        reports
            .slot_uses
            .keys()
            .filter(|port| matches!(port, Port::Operation(_)))
            .count(),
        2
    );
    assert_eq!(
        reports
            .slot_uses
            .keys()
            .filter(|port| matches!(port, Port::Emission(_)))
            .count(),
        2
    );
    assert!(reports.slot_uses.values().any(|(owner, _)| *owner == 1));
    for (owner, slot) in reports.slot_uses.values() {
        assert_eq!(checker.bodies[&slot.block].owner, *owner);
    }
}
