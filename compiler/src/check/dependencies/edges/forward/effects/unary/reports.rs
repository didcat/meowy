use super::{super::tests::checked, *};

#[test]
pub(crate) fn unary_effects_preserve_integer_widths_signedness_and_raw_types() {
    for signed in [false, true] {
        for bits in [8, 16, 32, 64] {
            let name = if signed { "int" } else { "uint" };
            let mut source = format!("b:@\"bits\";n<{name}{bits}>:=7;x:b.not(n)");
            if signed {
                source.push_str(&format!(";y<{name}{bits}><null>:-n"));
            }
            crate::compile(&source).unwrap();
            let (checker, reports) = checked(&source, false);
            assert_eq!(checker.unaries.len(), 1 + usize::from(signed));
            for (&id, op) in &checker.unaries {
                let (_, Effect::Unary(unary)) = &reports.effects[&id] else {
                    panic!()
                };
                assert_eq!(unary.input, op.input);
                assert_eq!(unary.op, op.kind);
                assert_eq!(unary.ty, ScalarKind::Int { bits, signed });
                assert_eq!(unary.checked, op.kind == UnaryKind::Negate);
                assert!(!unary.projected && unary.operation && unary.result);
            }
        }
    }
}

#[test]
pub(crate) fn unary_effects_keep_additional_and_expected_projections_distinct() {
    let source = "flag:false;|flag|x:-({->2;->tag:true});f<null>:(){-({->2;->tag:true})};r:{->3;->tag:true};y:-r";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.unaries.len(), 3);
    assert_eq!(checker.unaries.values().filter(|op| op.primary).count(), 2);
    assert_eq!(checker.unaries.values().filter(|op| op.control).count(), 1);
    assert!(checker.unaries.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.unaries {
        let (owner, Effect::Unary(unary)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(unary.control, op.control);
        assert_eq!(unary.primary, op.primary);
        assert_eq!(unary.projected, op.primary);
        assert!(unary.operation && unary.result);
        if !op.primary {
            assert!(checker.coercions[&op.input].primary);
        }
    }
}

#[test]
pub(crate) fn unary_effects_exclude_required_literals_stopped_inputs_and_later_operations() {
    let source = "n<int8>:-128;<T>:{x:-2;-><int32[-x]>};xs<T>:[1];stop<never>:(){'loop{'loop.restart()}};!stop();later:!false";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.unaries.len(), 3);
    for id in checker.unaries.keys() {
        assert!(!reports.effects.contains_key(id));
    }
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Unary(_)))
    );
}

#[test]
pub(crate) fn unary_effects_keep_operation_and_checked_result_observations_independent() {
    let (mut checker, mut reports) = checked("x:-({->2;->tag:true})", false);
    let id = *checker.unaries.first_key_value().unwrap().0;
    for (port, projected, operation, result) in [
        (Port::Projection { point: id, step: 0 }, true, false, false),
        (Port::Operation(id), false, true, false),
        (Port::Normal(id), false, false, true),
    ] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
        let effects = checker
            .operation_effects(&reports, Span::default())
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Unary(unary)) = &effects[&id] else {
            panic!()
        };
        assert!(unary.primary && unary.checked);
        assert_eq!(unary.projected, projected);
        assert_eq!(unary.operation, operation);
        assert_eq!(unary.result, result);
    }
}
