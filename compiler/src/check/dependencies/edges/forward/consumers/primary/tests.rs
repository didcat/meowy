use super::{super::tests::checked, *};

#[test]
pub(crate) fn primary_slot_links_keep_exact_unary_and_binary_projection_ports() {
    for source in [
        "a:-{->7;->tag:true}",
        "a:{->7;->tag:true}+1",
        "a:{->1;->tag:false}==1",
    ] {
        let (checker, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), 1);
        let (&port, (owner, slot)) = reports.slot_uses.first_key_value().unwrap();
        let Port::Projection { point, step: 0 } = port else {
            panic!()
        };
        let input = match &reports.effects[&point].1 {
            Effect::Unary(op) => {
                assert!(op.projected);
                op.input
            }
            Effect::Binary(op) => {
                assert!(op.projected[0]);
                op.inputs[0]
            }
            _ => panic!(),
        };
        assert_eq!(slot.index, 0);
        assert_eq!(checker.bodies[&slot.block].owner, *owner);
        assert_eq!(reports.results[&slot.block].1.consumer, Some(input));
    }
}

#[test]
pub(crate) fn primary_slot_links_preserve_projection_only_observation_before_stop() {
    let source = "d:@\"debug\";v:{->1;->tag:false}+{d.panic(\"stop\")}";
    let (mut checker, mut reports) = checked(source);
    let (&port, _) = reports.slot_uses.first_key_value().unwrap();
    let Port::Projection { point, step: 0 } = port else {
        panic!()
    };
    let Effect::Binary(op) = &reports.effects[&point].1 else {
        panic!()
    };
    assert_eq!(op.projected, [true, false]);
    assert!(!op.operation && !op.result);
    let prior = reports.slot_uses.clone();
    let Effect::Binary(op) = &mut reports.effects.get_mut(&point).unwrap().1 else {
        panic!()
    };
    op.operation = true;
    assert!(checker.slot_uses(&reports, Span::default()).is_err());
    assert_eq!(reports.slot_uses, prior);
}

#[test]
pub(crate) fn primary_slot_links_do_not_infer_projection_visits_or_unwrap_other_sources() {
    for source in [
        "r:{->1;->tag:true};x:-r",
        "x:-({->1;->tag:true})",
        "x:-{->1}",
        "r:{->1;->tag:true};x<int32>:r",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (mut checker, mut reports) = checked("x:-{->1;->tag:true}");
    for (_, effect) in reports.effects.values_mut() {
        if let Effect::Unary(op) = effect {
            op.projected = false;
        }
    }
    let effects = reports.effects.clone();
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap()
            .is_empty()
    );
    assert_eq!(reports.effects, effects);
}
