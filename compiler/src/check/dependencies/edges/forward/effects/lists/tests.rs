use super::{super::tests::checked, *};

#[test]
pub(crate) fn list_effects_keep_source_order_capacity_and_per_input_coercions() {
    for source in [
        "n<uint8>:2;xs:[1,n,3];empty<int32[4]>:[];nested:[[1],[2]]",
        "<T>:<int32><boolean>;r:{->1;->tag:true};xs<T[2]>:[r,2]",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        for (&id, op) in &checker.lists {
            let (owner, Effect::List(observed)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(*owner, op.owner);
            assert_eq!(observed.capacity, op.capacity);
            assert_eq!(observed.normal, op.normal);
            assert!(!observed.contextual && !observed.control);
            assert!(observed.constructed && observed.result);
            assert_eq!(observed.inputs.len(), op.count);
            for (input, point) in observed
                .inputs
                .iter()
                .zip(&checker.sequences[&SequenceSource::Expr(id)].items)
            {
                assert_eq!(Some(input.point), *point);
                assert!(input.plan.is_none() && !input.projected && !input.converted);
            }
        }
        if source.contains("r,2") {
            assert!(reports.effects.values().any(
                |(_, effect)| matches!(effect, Effect::Coercion(op) if op.projected && op.operation)
            ));
        }
    }
}

#[test]
pub(crate) fn list_effects_keep_projection_conversion_construction_and_result_visits_independent() {
    let source = "<T>:<int32><boolean>;<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<T[1]><string[1]>:[v]}";
    let (mut checker, mut reports) = checked(source, false);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let (_, Effect::List(observed)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(observed.inputs[0].plan, Some((true, CoercionKind::Convert)));
    assert!(observed.inputs[0].projected && observed.inputs[0].converted);
    assert!(observed.constructed && observed.result);
    for port in [
        Port::Projection { point: id, step: 0 },
        Port::Conversion { point: id, part: 0 },
        Port::Operation(id),
        Port::Normal(id),
    ] {
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.clear();
        }
        reports.entries.get_mut(&owner).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 3, 0)
            .unwrap();
        let (_, Effect::List(op)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(
            op.inputs[0].projected,
            port == Port::Projection { point: id, step: 0 }
        );
        assert_eq!(
            op.inputs[0].converted,
            port == Port::Conversion { point: id, part: 0 }
        );
        assert_eq!(op.constructed, port == Port::Operation(id));
        assert_eq!(op.result, port == Port::Normal(id));
    }
}

#[test]
pub(crate) fn list_effects_keep_partial_prefixes_and_disconnected_suffix_records() {
    for (source, prefix, projected) in [
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs<int32[2]>:[stop(),1]",
            false,
            false,
        ),
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs<int32[2]><uint8[2]>:[stop(),{x:300;->x}]",
            false,
            false,
        ),
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};<T>:<int32><boolean>;f:(v<int32><string>){|v<int32>|xs<T[3]><string[3]>:[v,stop(),{x:true;->x}]}",
            true,
            false,
        ),
        (
            "<R>:<{-><never>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[2]><string[2]>:[v,{x:1;->x}]}",
            true,
            true,
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let (&id, op) = checker.lists.first_key_value().unwrap();
        assert_eq!(reports.effects.contains_key(&id), prefix);
        if prefix {
            let (_, Effect::List(observed)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(observed.inputs.len(), op.count);
            assert_eq!(observed.inputs[0].projected, projected);
            assert_eq!(observed.inputs[0].converted, !projected);
            assert!(!observed.constructed && !observed.result);
            let last = observed.inputs.last().unwrap();
            assert!(!last.projected && !last.converted);
        }
        let last = checker.sequences[&SequenceSource::Expr(id)]
            .items
            .last()
            .unwrap()
            .unwrap();
        assert!(
            !reports.entries[&op.owner]
                .1
                .ports
                .contains(&Port::Entry(last))
        );
    }
}
