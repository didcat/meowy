use super::{super::tests::checked, *};

#[test]
pub(crate) fn narrowing_effects_preserve_guarded_local_and_field_roots() {
    let source =
        "f:(v<int32><null>,r<{n<int32><null>}>){|v<int32>|a<int32>:v;|r.n<int32>|b<int32>:r.n}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(
        checker.narrowings.values().filter(|op| op.changed).count(),
        2
    );
    for (&id, op) in &checker.narrowings {
        let (owner, Effect::Narrowing(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_ne!(*owner, 0);
        assert_eq!(observed.input, op.input);
        assert_eq!(observed.changed, op.changed);
        assert_eq!(observed.normal, op.normal);
        assert_eq!(observed.operation, op.changed);
        assert!(observed.result);
        assert!(matches!(
            reports.effects[&op.input].1,
            Effect::Read { .. } | Effect::Field { .. }
        ));
        assert_eq!(reports.index.operations.contains_key(&id), op.changed);
    }
}

#[test]
pub(crate) fn narrowing_effects_keep_receiver_effects_outer_contexts_and_control() {
    let source = "flag:false;f<{n<int32>}>:(){->{->n:1}};x<int32><null>:f().n;v<int32><null>:=1;|flag|v;|v<int32>|a:v";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert!(checker.narrowings.values().any(|op| op.control));
    assert!(!checker.proofs.observations.is_empty());
    let (&id, op) = checker
        .narrowings
        .iter()
        .find(|(_, op)| checker.fields.contains_key(&op.input))
        .unwrap();
    let field = &checker.fields[&op.input];
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(field.input, call.point);
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    assert!(!op.changed);
    let outer = checker.points[id].parent.unwrap();
    assert_eq!(checker.coercions[&outer].input, id);
    assert_eq!(
        checker.coercions[&outer].kind,
        crate::check::dependencies::CoercionKind::Convert
    );
    for (&id, op) in &checker.narrowings {
        let (_, Effect::Narrowing(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(observed.control, op.control);
    }
}

#[test]
pub(crate) fn narrowing_effects_keep_forwarding_conversion_and_result_visits_independent() {
    for source in ["v:1;v", "f:(v<int32><null>){|v<int32>|v}"] {
        let (mut checker, mut reports) = checked(source, false);
        let (&id, op) = checker
            .narrowings
            .iter()
            .find(|(_, op)| op.changed)
            .or_else(|| checker.narrowings.first_key_value())
            .unwrap();
        let owner = op.owner;
        let changed = op.changed;
        for port in [Port::Operation(id), Port::Normal(id)] {
            reports.entries.get_mut(&owner).unwrap().1.ports = vec![port, port];
            let result = checker.operation_effects(&reports, Span::default());
            if port == Port::Operation(id) && !changed {
                assert!(result.is_err());
                continue;
            }
            let effects = result.unwrap();
            let (_, Effect::Narrowing(op)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(op.operation, port == Port::Operation(id));
            assert_eq!(op.result, port == Port::Normal(id));
        }
        reports.index.operations.remove(&id);
        assert_eq!(
            checker
                .narrowing_effect_stage(&reports, owner, Port::Normal(id), Span::default())
                .is_ok(),
            !changed
        );
    }
}

#[test]
pub(crate) fn narrowing_effects_exclude_required_reads_and_direct_never_results() {
    let (checker, reports) = checked("<T>:{n:2;-><int32[n]>}", false);
    assert!(checker.narrowings.is_empty());
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Narrowing(_)))
    );
    let source = "f<never>:(v<never>){->v};g<never>:(r<{n<never>}>){->r.n}";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, false);
    let stopped: Vec<_> = checker
        .narrowings
        .iter()
        .filter(|(_, op)| !op.normal)
        .map(|(&id, op)| (id, op.owner))
        .collect();
    assert_eq!(stopped.len(), 2);
    for (id, owner) in stopped {
        assert!(!reports.effects.contains_key(&id));
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(
                checker
                    .narrowing_effect_stage(&reports, owner, port, Span::default())
                    .is_err()
            );
        }
    }
}
