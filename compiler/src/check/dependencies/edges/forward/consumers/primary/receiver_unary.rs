use super::{super::tests::checked, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn receiver_unary_primaries_keep_ascribed_ports_shapes_and_independent_owners() {
    for (init, dispatch) in [("{->3;->tag:true}", false), ("3.{->$;->tag:true}", true)] {
        let source = format!("<R>:<{{-><int32>;tag<boolean>}}>;
            r:{init};out:r.{{bare:-$;owned:-((($~<R>)));alias:(($));again:-(alias~<R>);inner:$.{{owned:-($~<R>)}}}};
            f<null>:(){{r:{init};out:r.{{owned:-($~<R>)}}}}");
        let (mut checker, reports) = checked(&source);
        let ids: Vec<_> = checker
            .unaries
            .iter()
            .filter_map(|(&id, op)| op.primary.then_some(id))
            .collect();
        assert_eq!(ids.len(), 4);
        assert_eq!(checker.unaries.len(), 5);
        let mut owners = BTreeSet::new();
        for id in ids {
            let op = &checker.unaries[&id];
            let (_, Effect::Unary(unary)) = &reports.effects[&id] else {
                panic!()
            };
            assert!(unary.primary && unary.projected);
            let (owner, slot) = reports.slot_uses[&Port::Projection { point: id, step: 0 }];
            assert_eq!((owner, slot.index), (op.owner, 0));
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(slots[0].shape, Shape::Scalar(unary.ty));
            assert_eq!(reports.results[&slot.block].1.dispatch.is_some(), dispatch);
            owners.insert(owner);
        }
        assert_eq!(owners, BTreeSet::from([0, 1]));
        assert_eq!(reports.slot_uses.len(), 5);
        assert_eq!(reports.slot_uses.keys().filter(|port| matches!(port,
            Port::Projection { point, step: 0 } if matches!(reports.effects[point].1, Effect::Coercion(_)))).count(), 1);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn receiver_unary_primaries_keep_operator_kinds_widths_and_guarded_scopes() {
    for (ty, value, expr) in [
        ("int8", "7", "-($~<R>)"),
        ("int64", "7", "-($~<R>)"),
        ("float32", "1.5", "-($~<R>)"),
        ("float64", "1.5", "-($~<R>)"),
        ("boolean", "true", "!($~<R>)"),
        ("uint8", "7", "b.not($~<R>)"),
    ] {
        let source = format!(
            "<R>:<{{-><{ty}>;tag<boolean>}}>;
            b:@\"bits\";n<{ty}>:{value};r:n.{{->$;->tag:true}};out:r.{{x:{expr}}}"
        );
        let (checker, reports) = checked(&source);
        let (&id, op) = checker.unaries.first_key_value().unwrap();
        assert!(op.primary, "{source}");
        assert_eq!(reports.slot_uses.len(), 1, "{source}");
        assert!(
            reports
                .slot_uses
                .contains_key(&Port::Projection { point: id, step: 0 })
        );
    }
    let (checker, reports) = checked(
        "<R>:<{-><int32>;tag<boolean>}>;
        flag:=true;r:{->3;->tag:true};out:r.{|flag|x:-($~<R>);|!flag|$.{x:-($~<R>)}}",
    );
    assert_eq!(reports.slot_uses.len(), 2);
    for (&id, op) in &checker.unaries {
        assert_eq!(
            reports
                .slot_uses
                .contains_key(&Port::Projection { point: id, step: 0 }),
            op.primary
        );
    }
}
