use super::{super::tests::checked, *};
use crate::check::dependencies::CoercionKind;

mod boundaries;

#[test]
pub(crate) fn dispatch_coercion_primaries_retain_preconversion_types_ports_and_owners() {
    let source = "r:3.{->$;->tag:true};copy:((r));a<int32>:copy;b<int32><null>:copy;c:-copy;f<int32>:(){r:4.{->$;->tag:false};->r}";
    let (mut checker, reports) = checked(source);
    assert_eq!(reports.slot_uses.len(), 4);
    assert!(reports.slot_uses.values().any(|(owner, _)| *owner != 0));
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut converted = false;
    for (&port, &(owner, slot)) in &reports.slot_uses {
        let Port::Projection { point, step: 0 } = port else {
            panic!()
        };
        let (_, Effect::Coercion(op)) = &reports.effects[&point] else {
            panic!()
        };
        assert!(op.primary && op.projected);
        assert_eq!(slot.index, 0);
        let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
            panic!()
        };
        assert_eq!(op.source, Some(slots[0].shape));
        let dispatch = reports.results[&slot.block].1.dispatch.unwrap();
        assert_eq!(checker.dispatch_ops[&dispatch].owner, owner);
        assert!(!reports.consumers.contains_key(&dispatch));
        assert_eq!(
            checker
                .forward_coercion_input(&reports, point, owner, Span::default())
                .unwrap(),
            None
        );
        converted |= op.op == CoercionKind::Convert;
    }
    assert!(converted);
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Unary(op) if !op.primary && !op.projected))
    );
    assert!(
        reports
            .direct_sources
            .values()
            .all(|(_, direct)| direct.source.is_none()
                && direct.block.is_none()
                && direct.dispatch.is_none())
    );
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn dispatch_coercion_primaries_keep_projection_conversion_and_result_visits_independent()
{
    let (mut checker, mut reports) = checked("r:3.{->$;->tag:true};x<int32><null>:r");
    let id = *checker
        .coercions
        .iter()
        .find(|(_, op)| op.primary)
        .unwrap()
        .0;
    let dispatch = *checker.dispatch_ops.keys().next().unwrap();
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 1);
    for initialized in [false, true] {
        for completed in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = completed;
            for (projected, operation, result) in [
                (true, false, false),
                (true, true, true),
                (false, true, false),
                (false, false, true),
            ] {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.projected = projected;
                op.operation = operation;
                op.result = result;
                assert_eq!(
                    checker.slot_uses(&reports, Span::default()).unwrap(),
                    if projected && completed {
                        expected.clone()
                    } else {
                        Uses::new()
                    }
                );
            }
        }
    }
    let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.projected = true;
    op.operation = false;
    op.result = false;
    reports.index.operations.remove(&id);
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.operation = true;
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn dispatch_coercion_primaries_keep_nonscalar_stopped_and_other_producers_opaque() {
    for source in [
        "r:3.{->[$];->tag:true};x<int32[1]>:r",
        "n:1;r:3.{->&n;->tag:true};x<&int32>:r",
        "n<int32><null>:1;r:3.{->n;->tag:true};x<int32><null>:r",
        "r:3.{->$;->tag:true};x<{-><int32>;tag<boolean>}><null>:r",
        "r:=3.{->$;->tag:true};x<int32>:r",
        "r:3.{->$;->tag:true};p:&r;x<int32>:*p",
        "f<{-><int32>;tag<boolean>}>:(){->3;->tag:true};x<int32>:f()",
        "f<boolean>:(r<{-><never>;tag<boolean>}>){->r}",
        "d:@\"debug\";x<int32>:d.panic(\"stop\")",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn dispatch_coercion_primaries_keep_nested_list_coercions_owned_by_their_own_ports() {
    let (checker, reports) =
        checked("r:3.{->$;->tag:true};xs<int32[1]>:[r];d:@\"debug\";d.print(r);out:{->r}");
    assert_eq!(reports.slot_uses.len(), 1);
    let (&port, _) = reports.slot_uses.first_key_value().unwrap();
    let Port::Projection { point, step: 0 } = port else {
        panic!()
    };
    assert!(
        matches!(&reports.effects[&point].1, Effect::Coercion(op) if op.primary && op.projected)
    );
    let parent = checker.points[point].parent.unwrap();
    let (_, Effect::List(list)) = &reports.effects[&parent] else {
        panic!()
    };
    assert_eq!(list.inputs[0].point, point);
    assert!(list.inputs[0].plan.is_none() && !list.inputs[0].projected);
    assert!(!reports.slot_uses.contains_key(&Port::Projection {
        point: parent,
        step: 0
    }));
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Output(_)))
    );
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Emission(_)))
    );
}
