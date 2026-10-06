use super::{super::tests::checked, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn receiver_binary_primaries_keep_operand_positions_types_and_independent_owners() {
    for (init, dispatch) in [("{->3;->tag:true}", false), ("3.{->$;->tag:true}", true)] {
        let source = format!(
            "r:{init};out:r.{{alias:(($));left:1+alias;right:alias-1;both:alias+alias;test:alias<4;eq:3==alias;inner:$.{{->$+1}};nested:{{->alias+2}}}};f<int32>:(){{r:{init};->r.{{->$+1}}}}"
        );
        let (mut checker, reports) = checked(&source);
        let mut owners = BTreeSet::new();
        let mut count = 0;
        for (&id, op) in &checker.binaries {
            let (_, Effect::Binary(binary)) = &reports.effects[&id] else {
                panic!()
            };
            for (step, projected) in binary.projected.into_iter().enumerate() {
                let port = Port::Projection { point: id, step };
                if !projected {
                    assert!(!reports.slot_uses.contains_key(&port));
                    continue;
                }
                count += 1;
                let (owner, slot) = reports.slot_uses[&port];
                assert_eq!((owner, slot.index), (op.owner, 0));
                let BinaryClass::Scalar(ty) = binary.types.inputs[step] else {
                    panic!()
                };
                let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(slots[0].shape, Shape::Scalar(ty));
                assert_eq!(reports.results[&slot.block].1.dispatch.is_some(), dispatch);
                owners.insert(owner);
            }
        }
        assert_eq!(count, 9);
        assert_eq!(reports.slot_uses.len(), count);
        assert_eq!(owners, BTreeSet::from([0, 1]));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn receiver_binary_primaries_keep_scalar_kinds_and_guarded_receiver_scopes() {
    for (ty, value, expr) in [
        ("uint8", "7", "$%2"),
        ("float32", "1.5", "2.0/$"),
        ("boolean", "true", "$==true"),
        ("string", "\"cat\"", "$<\"dog\""),
        ("null", "null", "null==$"),
    ] {
        let source = format!("n<{ty}>:{value};r:n.{{->$;->tag:true}};out:r.{{x:{expr}}}");
        let (_, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 1, "{source}");
        let &Port::Projection { point, step } = reports.slot_uses.keys().next().unwrap() else {
            panic!()
        };
        let (_, Effect::Binary(op)) = &reports.effects[&point] else {
            panic!()
        };
        assert!(op.projected[step]);
    }
    let (checker, reports) =
        checked("flag:=true;r:{->3;->tag:true};out:r.{|flag|->$+1;|!flag|->$.{->$+2}}");
    assert_eq!(reports.slot_uses.len(), 2);
    for &id in checker.binaries.keys() {
        assert!(
            reports
                .slot_uses
                .contains_key(&Port::Projection { point: id, step: 0 })
        );
    }
}
