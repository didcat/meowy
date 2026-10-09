use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn shared_receiver_binaries_link_types_and_guarded_owners() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        for init in ["{->p;->tag:true}", "p.{->$;->tag:true}"] {
            let source = format!(
                "n<{ty}>:{value};p:&n;flag:=true;x:({init}).{{a:$==p;b:p!=(($));|flag|inner:$.{{->$==p}}}};f<boolean>:(p<&{ty}>){{->({init}).{{->$==p}}}}"
            );
            let (mut checker, reports) = checked(&source);
            assert_eq!(reports.slot_uses.len(), 4, "{source}");
            assert!(reports.slot_uses.values().any(|(owner, _)| *owner != 0));
            for (&port, &(owner, slot)) in &reports.slot_uses {
                let Port::Projection { point, step } = port else {
                    panic!()
                };
                let (_, effect) = &reports.effects[&point];
                let Effect::Binary(op) = effect else {
                    assert!(matches!(effect, Effect::Coercion(op) if op.projected));
                    continue;
                };
                let BinaryClass::SharedScalar(kind) = op.types.inputs[step] else {
                    panic!()
                };
                assert!(op.projected[step] && op.operation && op.result);
                assert_eq!((slot.index, checker.bodies[&slot.block].owner), (0, owner));
                let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(slots[0].shape, Shape::SharedScalar(kind));
                assert_eq!(
                    reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                    Sources::Unknown
                );
            }
            let projected: Vec<_> = checker
                .coercions
                .iter()
                .filter(|(_, op)| matches!(op.source, Some(Shape::SharedScalar(_))))
                .collect();
            assert_eq!(projected.len(), 1);
            let (&id, _) = projected[0];
            assert!(
                reports
                    .slot_uses
                    .contains_key(&Port::Projection { point: id, step: 0 })
            );
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
}

#[test]
pub(crate) fn shared_receiver_binaries_keep_source_results_initialization_and_projection_independent()
 {
    let (mut checker, mut reports) = checked("n:1;p:&n;x:p.{->$;->tag:true}.{->$==p}");
    let id = *checker.binaries.keys().next().unwrap();
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 1);
    let block = expected.values().next().unwrap().1.block;
    let source = reports.results[&block].1.dispatch.unwrap();
    let receiver = *checker
        .dispatch_ops
        .iter()
        .find(|(_, op)| op.shared_primary.is_some())
        .unwrap()
        .0;
    for completed in [false, true] {
        let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
            panic!()
        };
        op.result = completed;
        for initialized in [false, true] {
            for result in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.initialized = initialized;
                op.result = result;
                for (projected, operation, result) in [
                    (true, false, false),
                    (true, true, true),
                    (false, true, false),
                    (false, false, true),
                ] {
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.projected = [projected, false];
                    op.operation = operation;
                    op.result = result;
                    if !operation && !result {
                        reports.index.operations.remove(&id);
                    } else {
                        reports.index.operations.insert(id, 0);
                    }
                    assert_eq!(
                        checker.slot_uses(&reports, Span::default()).unwrap(),
                        if completed && initialized && projected {
                            expected.clone()
                        } else {
                            Uses::new()
                        }
                    );
                }
            }
        }
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn shared_receiver_binaries_preserve_stopped_order_and_other_consumer_boundaries() {
    let (checker, reports) =
        checked("d:@\"debug\";n:1;p:&n;x:p.{->$;->tag:true}.{->$==d.panic(\"stop\")}");
    let (&id, _) = checker.binaries.first_key_value().unwrap();
    let (_, Effect::Binary(op)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(op.projected, [true, false]);
    assert!(!op.operation && !op.result);
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(
        reports
            .slot_uses
            .contains_key(&Port::Projection { point: id, step: 0 })
    );
    for source in [
        "n:1;p:&n;r:{->p;->tag:true};x:r.{->$==p}",
        "n:1;p:&n;x:{->p;->tag:=true}.{->$==p}",
        "n:1;p:&n;x:{->p;->other:p}.{->$==p}",
        "n:1;p:&n;x:p.{->$;->tag:true}.{copy:$;->copy==p}",
        "n:1;p:&n;r:{->p;->tag:true};x:(&r).{->(*$)==p}",
        "n:1;p:&n;x:p.{->$;->tag:true}.{->$.tag}",
        "n:1;p:&n;x:p.{->$;->tag:true}.{->$==$}",
        "d:@\"debug\";n:1;p:&n;x:p.{->$;->tag:true}.{d.panic(\"stop\");->$==p}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
