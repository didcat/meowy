use super::{super::tests::checked, *};

#[test]
pub(crate) fn list_effects_preserve_control_function_owners_and_conditional_calls() {
    let source = "flag:false;g<int32>:(){->7};|flag|xs:[g(),1];f:(){nested:[[2]]}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert!(checker.lists.values().any(|op| op.control));
    assert!(checker.lists.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.lists {
        let (owner, Effect::List(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.control, op.control);
        assert!(observed.constructed && observed.result);
    }
    for call in checker.invocations.values() {
        assert!(matches!(
            reports.effects[&call.point].1,
            Effect::Call { .. }
        ));
        assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    }
}

#[test]
pub(crate) fn list_effects_preserve_deferred_contextual_slots_and_source_errors() {
    let source = "<T>:<uint8><boolean>;<U>:<uint16><boolean>;f:(v<uint8><uint16>){|v<uint8>|xs<T[3]><U[3]>:[1,v,3]}";
    let (checker, reports) = checked(source, false);
    let id = *checker.lists.first_key_value().unwrap().0;
    let (_, Effect::List(observed)) = &reports.effects[&id] else {
        panic!()
    };
    assert!(observed.inputs[0].point > observed.inputs[1].point);
    for (part, (input, text)) in observed.inputs.iter().zip(["1", "v", "3"]).enumerate() {
        let point = &checker.points[input.point];
        assert_eq!(&source[point.span.start..point.span.end], text);
        assert!(!input.projected);
        assert_eq!(input.converted, part == 1);
        assert_eq!(
            input.plan,
            Some((
                false,
                if part == 1 {
                    CoercionKind::Convert
                } else {
                    CoercionKind::Forward
                }
            ))
        );
    }
    for (source, code) in [
        ("xs:[]", "E207"),
        ("xs:[1,\"two\"]", "E207"),
        ("xs<uint8[1]><uint16[1]>:[1]", "E207"),
        ("xs<int32[1]>:[1,2]", "E103"),
        ("xs<uint8[1]>:[256]", "E216"),
        ("xs<never[1]>:[]", "B001"),
        ("n:1;xs:[&n]", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn list_effects_reject_contextual_plan_and_stage_corruption_atomically() {
    let source = "<T>:<int32><boolean>;<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<T[2]><string[2]>:[v,{x:true;->x}]}";
    for stage in 0..4 {
        for fault in 0..18 {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, op) = checker.lists.first_key_value().unwrap();
            let owner = op.owner;
            let key = SequenceSource::Expr(id);
            let mut port = match stage {
                0 => Port::Projection { point: id, step: 0 },
                1 => Port::Conversion { point: id, part: 0 },
                2 => Port::Operation(id),
                _ => Port::Normal(id),
            };
            match fault {
                0 => {
                    checker.list_inputs.remove(&id);
                }
                1 => {
                    checker.list_inputs.get_mut(&id).unwrap().pop();
                }
                2 => checker.list_inputs.get_mut(&id).unwrap()[0].point = id,
                3 => checker.list_inputs.get_mut(&id).unwrap()[0].primary = false,
                4 => checker.list_inputs.get_mut(&id).unwrap()[0].kind = CoercionKind::Forward,
                5 => checker.list_inputs.get_mut(&id).unwrap()[0].kind = CoercionKind::Stopped,
                6 => checker.list_inputs.get_mut(&id).unwrap()[1].kind = CoercionKind::Stopped,
                7 => checker.list_inputs.get_mut(&id).unwrap().swap(0, 1),
                8 => {
                    checker.endpoints.get_mut(&key).unwrap()[1].to =
                        Port::Projection { point: id, step: 1 }
                }
                9 => checker.endpoints.get_mut(&key).unwrap()[2].route = Route::Returned,
                10 => checker.sequences.get_mut(&key).unwrap().edges[0].route = Route::Checked,
                11 => {
                    checker.endpoints.remove(&key);
                }
                12 => {
                    checker.sequences.remove(&key);
                }
                13 => checker.lists.get_mut(&id).unwrap().normal = false,
                14 => port = Port::Projection { point: id, step: 2 },
                15 => port = Port::Conversion { point: id, part: 1 },
                16 => {
                    reports.index.operations.remove(&id);
                }
                17 => {
                    reports.index.operations.insert(id, owner + 1);
                }
                _ => unreachable!(),
            }
            for (_, walk) in reports.entries.values_mut() {
                walk.ports.clear();
            }
            reports.entries.get_mut(&owner).unwrap().1.ports = vec![port];
            let before = reports.effects.clone();
            let lists = checker.lists.clone();
            let plans = checker.list_inputs.clone();
            assert!(
                checker
                    .operation_effects(&reports, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "stage {stage} fault {fault}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.lists, lists);
            assert_eq!(checker.list_inputs, plans);
        }
    }
}
