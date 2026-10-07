use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn receiver_list_primaries_keep_sparse_stages_sources_and_initialization_independent() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 3);
    let slot = expected.values().next().unwrap().1;
    let source = reports.results[&slot.block].1.dispatch.unwrap();
    let receiver = *checker
        .dispatch_ops
        .iter()
        .find(|(_, op)| matches!(op.receiver, Shape::Record { .. }))
        .unwrap()
        .0;
    let full = reports.effects[&id].1.clone();
    for published in [false, true] {
        let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
            panic!()
        };
        op.result = published;
        op.initialized = !published;
        for initialized in [false, true] {
            for completed in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.initialized = initialized;
                op.result = completed;
                for (projected, converted, constructed, result) in [
                    ([true, false, false], false, false, false),
                    ([false, true, false], false, false, false),
                    ([false, false, true], false, false, false),
                    ([true; 3], true, true, true),
                    ([false; 3], true, false, false),
                    ([false; 3], false, true, false),
                    ([false; 3], false, false, true),
                ] {
                    let Effect::List(mut list) = full.clone() else {
                        panic!()
                    };
                    list.constructed = constructed;
                    list.result = result;
                    for (step, input) in list.inputs.iter_mut().enumerate() {
                        input.projected = [0, 2, 3]
                            .iter()
                            .position(|&part| part == step)
                            .is_some_and(|part| projected[part]);
                        input.converted = converted && step == 3;
                    }
                    reports.effects.get_mut(&id).unwrap().1 = Effect::List(list);
                    let selected = expected
                        .iter()
                        .filter_map(|(&port, &value)| {
                            let Port::Projection { step, .. } = port else {
                                panic!()
                            };
                            let part = [0, 2, 3].iter().position(|&part| part == step).unwrap();
                            (projected[part] && published && initialized).then_some((port, value))
                        })
                        .collect::<Uses>();
                    assert_eq!(
                        checker.slot_uses(&reports, Span::default()).unwrap(),
                        selected
                    );
                    reports.index.operations.remove(&id);
                    let before = format!("{reports:?}{:?}", checker.edge_counts());
                    assert!(
                        checker
                            .slot_uses(&reports, Span::default())
                            .unwrap_err()
                            .message
                            .contains("identity")
                    );
                    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
                    reports.index.operations.insert(id, owner);
                }
            }
        }
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn receiver_list_primaries_keep_prefix_links_before_stopped_elements_and_bodies() {
    let prefix =
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};<T>:<int32><boolean>;<U>:<int32><string>";
    let source = format!(
        "{prefix};f:(v<boolean><string>){{r:3.{{->$;->tag:true}};out:r.{{|v<boolean>|xs<T[4]><U[4]>:[$,v,stop(),$];d.panic(\"body stop\")}}}}"
    );
    let (mut checker, reports) = checked(&source);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let (_, Effect::List(list)) = &reports.effects[&id] else {
        panic!()
    };
    assert!(!op.normal && !list.constructed && !list.result);
    assert!(list.inputs[0].projected && list.inputs[0].converted);
    assert!(!list.inputs[3].projected && !list.inputs[3].converted);
    assert!(checker.points[list.inputs[3].point].complete);
    assert!(!reports.index.operations.contains_key(&id));
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(
        reports
            .slot_uses
            .contains_key(&Port::Projection { point: id, step: 0 })
    );
    assert!(
        reports
            .effects
            .values()
            .any(|(_, op)| matches!(op, Effect::Dispatch(op) if op.initialized && !op.result))
    );
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    for body in [
        "|v<boolean>|xs<T[3]><U[3]>:[stop(),true,$]",
        "d.panic(\"stop\");|v<boolean>|xs<T[2]><U[2]>:[$,true]",
    ] {
        let (_, reports) = checked(&format!(
            "{prefix};f:(v<boolean><string>){{r:{{->3;->tag:true}};out:r.{{{body}}}}}"
        ));
        assert!(reports.slot_uses.is_empty());
    }
}

#[test]
pub(crate) fn receiver_list_primaries_keep_opaque_sources_and_inner_coercions_separate() {
    for (setup, input) in [
        ("r:={->3;->tag:true}", "r"),
        ("r:{->3;->tag:=true}", "r"),
        ("base:1;r:{->3;->tag:true;->p:&base}", "r"),
        ("r:{->3;->tag:true};p:&r", "*p"),
        ("get<{-><int32>;tag<boolean>}>:(){->3;->tag:true}", "get()"),
    ] {
        let source = format!(
            "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){{{setup};out:({input}).{{|v<boolean>|xs<T[2]><U[2]>:[$,v]}}}}"
        );
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (_, reports) = checked(
        "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>,r<{-><int32>;tag<boolean>}>){out:r.{|v<boolean>|xs<T[2]><U[2]>:[$,v]}}",
    );
    assert!(reports.slot_uses.is_empty());
    for (ty, setup) in [
        ("int32><null", "n<int32><null>:1;r:{->n;->tag:true}"),
        ("int32[1]", "r:{->[1];->tag:true}"),
    ] {
        let source = format!(
            "<T>:<{ty}><boolean>;<U>:<{ty}><string>;f:(v<boolean><string>){{{setup};out:r.{{|v<boolean>|xs<T[2]><U[2]>:[$,v]}}}}"
        );
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty());
    }
    let (checker, reports) = checked("r:{->3;->tag:true};out:r.{xs<int32[2]>:[$,1]}");
    assert_eq!(reports.slot_uses.len(), 1);
    let Port::Projection { point, step: 0 } = *reports.slot_uses.keys().next().unwrap() else {
        panic!()
    };
    assert!(matches!(reports.effects[&point].1, Effect::Coercion(_)));
    assert!(!checker.lists.contains_key(&point));
}

#[test]
pub(crate) fn receiver_list_primaries_preserve_empty_and_multiple_candidate_histories() {
    for (ty, init, count) in [
        ("null", "{->tag:true}", 0),
        ("int32", "{flag:=false;|flag|->3;|!flag|->4;->tag:true}", 2),
    ] {
        let source = format!(
            "<T>:<{ty}><boolean>;<U>:<{ty}><string>;f:(v<boolean><string>){{r:{init};out:r.{{|v<boolean>|xs<T[2]><U[2]>:[$,v]}}}}"
        );
        let (_, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 1);
        let slot = reports.slot_uses.values().next().unwrap().1;
        let Sources::Candidates(values) =
            &reports.results[&slot.block].1.slots.as_ref().unwrap()[0]
        else {
            panic!()
        };
        assert_eq!(values.len(), count);
    }
}
