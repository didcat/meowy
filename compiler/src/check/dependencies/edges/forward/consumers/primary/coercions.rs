use super::{super::tests::checked, *};
use crate::check::dependencies::CoercionKind;

#[test]
pub(crate) fn coercion_consumers_keep_projection_ports_inputs_and_independent_owners() {
    let source = "r:{->7;->tag:true};copy:((r));a<int32>:copy;b<int32><null>:copy;c:-copy;f<int32>:(){r:{->9;->tag:false};->r}";
    let (mut checker, reports) = checked(source);
    assert_eq!(reports.slot_uses.len(), 4);
    assert!(reports.slot_uses.values().any(|(owner, _)| *owner != 0));
    let mut kinds = Vec::new();
    for (&port, &(owner, slot)) in &reports.slot_uses {
        let Port::Projection { point, step: 0 } = port else {
            panic!()
        };
        let Effect::Coercion(op) = &reports.effects[&point].1 else {
            panic!()
        };
        assert!(op.primary && op.projected);
        assert_eq!(slot.index, 0);
        assert_eq!(checker.bodies[&slot.block].owner, owner);
        let anchor = checker
            .grouped_consumer(&reports, op.input, owner, Span::default())
            .unwrap()
            .unwrap();
        assert_eq!(reports.consumers[&anchor], (owner, slot.block));
        assert!(
            checker
                .forward_coercion_input(&reports, point, owner, Span::default())
                .unwrap()
                .is_none()
        );
        kinds.push(op.op);
    }
    assert!(kinds.contains(&CoercionKind::Forward));
    assert!(kinds.contains(&CoercionKind::Convert));
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Unary(op) if !op.primary && !op.projected))
    );
}

#[test]
pub(crate) fn coercion_consumers_require_observed_projection_and_preserve_other_stages() {
    let (mut checker, mut reports) = checked("r:{->7;->tag:true};x<int32><null>:r");
    let (&id, _) = checker.coercions.iter().find(|(_, op)| op.primary).unwrap();
    let prior = reports.slot_uses.clone();
    assert_eq!(prior.len(), 1);
    for state in 0..3 {
        let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.projected = state == 0;
        op.operation = state == 1;
        op.result = state == 2;
        let expected = if state == 0 {
            prior.clone()
        } else {
            Uses::new()
        };
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected
        );
    }
    assert_eq!(reports.slot_uses, prior);
}

#[test]
pub(crate) fn coercion_consumers_keep_parameters_references_and_calls_opaque() {
    for source in [
        "f<int32>:(r<{-><int32>;tag<boolean>}>){->r}",
        "r:={->7;->tag:true};x<int32>:r",
        "r:{->7;->tag:true};p:&r;x<int32>:*p",
        "f<{-><int32>;tag<boolean>}>:(){->7;->tag:true};x<int32>:f()",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
