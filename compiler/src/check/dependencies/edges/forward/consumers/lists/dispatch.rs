use super::*;

mod boundaries;

pub(super) const SOURCE: &str = "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:3.{->$;->tag:true};copy:((r));|v<boolean>|xs<T[5]><U[5]>:[r,1,copy,r,v]}";

#[test]
pub(crate) fn dispatch_list_primaries_keep_original_indices_shapes_and_independent_owners() {
    let source = format!(
        "{SOURCE};v<boolean><string>:true;r:4.{{->$;->tag:false}};|v<boolean>|xs<T[2]><U[2]>:[r,v]"
    );
    let (mut checker, reports) = checked(&source);
    assert_eq!(reports.slot_uses.len(), 4);
    let mut steps = BTreeMap::new();
    for (&port, &(owner, slot)) in &reports.slot_uses {
        let Port::Projection { point, step } = port else {
            panic!()
        };
        let (_, Effect::List(list)) = &reports.effects[&point] else {
            panic!()
        };
        assert!(list.contextual && list.inputs[step].projected);
        assert_eq!(list.inputs[step].plan, Some((true, CoercionKind::Convert)));
        assert_eq!(slot.index, 0);
        let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
            panic!()
        };
        assert_eq!(list.inputs[step].source, Some(slots[0].shape));
        let dispatch = reports.results[&slot.block].1.dispatch.unwrap();
        assert_eq!(checker.dispatch_ops[&dispatch].owner, owner);
        assert!(!reports.consumers.contains_key(&dispatch));
        steps.entry(owner).or_insert_with(Vec::new).push(step);
    }
    assert_eq!(steps[&0], [0]);
    assert_eq!(steps[&1], [0, 2, 3]);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    for (ty, setup) in [
        ("uint8", "n<uint8>:7;r:n.{->$;->tag:true}"),
        ("float32", "n<float32>:1.5;r:n.{->$;->tag:true}"),
        ("string", "r:\"cat\".{->$;->tag:true}"),
        ("null", "r:3.{->tag:true}"),
        ("boolean", "r:true.{->$;->tag:true}"),
    ] {
        let select = if ty == "boolean" { "int32" } else { "boolean" };
        let source = format!(
            "<T>:<{ty}><{select}>;<U>:<{ty}><string>;f:(v<{select}><string>){{{setup};|v<{select}>|xs<T[2]><U[2]>:[r,v]}}"
        );
        let (_, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 1, "{source}");
    }
}

#[test]
pub(crate) fn dispatch_list_primaries_keep_stage_and_source_result_visits_independent() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let dispatch = *checker.dispatch_ops.keys().next().unwrap();
    let port = Port::Projection { point: id, step: 3 };
    let expected = Uses::from([(port, reports.slot_uses[&port])]);
    let full = reports.effects[&id].1.clone();
    for initialized in [false, true] {
        for result in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = result;
            for state in 0..4 {
                let Effect::List(mut list) = full.clone() else {
                    panic!()
                };
                list.constructed = state == 2;
                list.result = state == 3;
                for (step, input) in list.inputs.iter_mut().enumerate() {
                    input.projected = state == 0 && step == 3;
                    input.converted = state == 1 && step == 3;
                }
                reports.effects.get_mut(&id).unwrap().1 = Effect::List(list);
                assert_eq!(
                    checker.slot_uses(&reports, Span::default()).unwrap(),
                    if state == 0 && result {
                        expected.clone()
                    } else {
                        Uses::new()
                    }
                );
                reports.index.operations.remove(&id);
                assert!(
                    checker
                        .slot_uses(&reports, Span::default())
                        .unwrap_err()
                        .message
                        .contains("identity")
                );
                reports.index.operations.insert(id, owner);
            }
        }
    }
}

#[test]
pub(crate) fn dispatch_list_primaries_keep_prefix_links_before_stopped_elements() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:3.{->$;->tag:true};|v<boolean>|xs<T[4]><U[4]>:[r,v,stop(),r]}";
    let (checker, reports) = checked(source);
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
}

#[test]
pub(crate) fn dispatch_list_primaries_keep_other_sources_and_inner_coercion_ports_distinct() {
    for (setup, input) in [
        ("r:=3.{->$;->tag:true}", "r"),
        ("r:3.{->$;->tag:true};p:&r", "*p"),
        ("get<{-><int32>;tag<boolean>}>:(){->3;->tag:true}", "get()"),
    ] {
        let source = format!(
            "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){{{setup};|v<boolean>|xs<T[2]><U[2]>:[{input},v]}}"
        );
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    for (ty, setup) in [
        ("int32><null", "n<int32><null>:1;r:3.{->n;->tag:true}"),
        ("int32[1]", "r:3.{->[$];->tag:true}"),
    ] {
        let source = format!(
            "<T>:<{ty}><boolean>;<U>:<{ty}><string>;f:(v<boolean><string>){{{setup};|v<boolean>|xs<T[2]><U[2]>:[r,v]}}"
        );
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty());
    }
    let (checker, reports) = checked("r:3.{->$;->tag:true};xs<int32[2]>:[r,1]");
    assert_eq!(reports.slot_uses.len(), 1);
    let Port::Projection { point, step: 0 } = *reports.slot_uses.keys().next().unwrap() else {
        panic!()
    };
    assert!(matches!(reports.effects[&point].1, Effect::Coercion(_)));
    assert!(!checker.lists.contains_key(&point));
}
