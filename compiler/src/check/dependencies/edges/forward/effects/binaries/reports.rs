use super::{super::tests::checked, *};

#[test]
pub(crate) fn binary_effects_preserve_widths_signedness_and_raw_result_types() {
    for signed in [false, true] {
        for bits in [8, 16, 32, 64] {
            let name = if signed { "int" } else { "uint" };
            let source = format!(
                "m:@\"bits\";a<{name}{bits}>:=7;b<{name}{bits}>:=2;x<{name}{bits}><null>:a+b;lt:a<b;z:m.xor(a,b)"
            );
            crate::compile(&source).unwrap();
            let (checker, reports) = checked(&source, false);
            assert_eq!(checker.binaries.len(), 3);
            let ty = Class::Scalar(ScalarKind::Int { bits, signed });
            for (&id, op) in &checker.binaries {
                let (_, Effect::Binary(binary)) = &reports.effects[&id] else {
                    panic!()
                };
                assert_eq!(binary.inputs, op.inputs);
                assert_eq!(binary.types.inputs, [ty; 2]);
                assert_eq!(
                    binary.types.result,
                    if op.op == "<" {
                        Class::Scalar(ScalarKind::Bool)
                    } else {
                        ty
                    }
                );
                assert_eq!(binary.plan.checked, op.op == "+");
                assert!(binary.operation && binary.result);
            }
        }
    }
}

#[test]
pub(crate) fn binary_effects_preserve_projection_plans_control_and_independent_owners() {
    let source = "flag:false;a:{->1;->tag:true};b:{->2;->tag:false};|flag|a+b;f<int32>:(x<int32>,y<int32>){->x+y}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.binaries.len(), 2);
    for (&id, op) in &checker.binaries {
        let (owner, Effect::Binary(binary)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(binary.control, *owner == 0);
        assert_eq!(binary.projected, op.plan.primary);
        assert_eq!(binary.projected, [*owner == 0; 2]);
        assert!(binary.operation && binary.result);
    }
}

#[test]
pub(crate) fn binary_effects_keep_partial_projections_and_runtime_exclusions() {
    for (tail, projected) in [("r+1", [true, false]), ("1+r", [false, true])] {
        let source = format!("f<never>:(r<{{-><never>;tag<boolean>}}> ){{->{tail}}}");
        let (checker, reports) = checked(&source, false);
        let id = *checker.binaries.first_key_value().unwrap().0;
        let (_, Effect::Binary(binary)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(binary.projected, projected);
        assert!(!binary.operation && !binary.result);
        assert_eq!(binary.types.result, Class::Never);
    }
    for source in [
        "stop<never>:(){'loop{'loop.restart()}};stop()+true",
        "<T>:{n:1+2;-><uint8[n]>};xs<T>:[1]",
        "false&&true",
    ] {
        crate::compile(source).unwrap();
        let (_, reports) = checked(source, false);
        assert!(
            !reports
                .effects
                .values()
                .any(|(_, effect)| matches!(effect, Effect::Binary(_)))
        );
    }
}

#[test]
pub(crate) fn binary_effects_keep_projection_operation_and_result_observations_independent() {
    let (mut checker, mut reports) = checked("a:{->1;->tag:true};b:{->2;->tag:false};a+b", false);
    let id = *checker.binaries.first_key_value().unwrap().0;
    for (port, projected, operation, result) in [
        (
            Port::Projection { point: id, step: 0 },
            [true, false],
            false,
            false,
        ),
        (
            Port::Projection { point: id, step: 1 },
            [false, true],
            false,
            false,
        ),
        (Port::Operation(id), [false; 2], true, false),
        (Port::Normal(id), [false; 2], false, true),
    ] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
        let effects = checker
            .operation_effects(&reports, Span::default())
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Binary(binary)) = &effects[&id] else {
            panic!()
        };
        assert!(binary.plan.checked);
        assert_eq!(binary.projected, projected);
        assert_eq!(binary.operation, operation);
        assert_eq!(binary.result, result);
    }
}
