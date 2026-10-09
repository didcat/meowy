use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::Sources;

mod boundaries;

#[test]
pub(crate) fn dispatch_reference_binaries_keep_exact_slots_and_operand_positions() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        for body in ["p.{->$;->tag:true}", "((p.{->$;->tag:true}))"] {
            let source = format!("n<{ty}>:{value};p:&n;x:{body}==p;y:p!={body}");
            let (mut checker, reports) = checked(&source);
            assert_eq!(reports.slot_uses.len(), 2, "{source}");
            for (&id, op) in &checker.binaries {
                let (_, Effect::Binary(binary)) = &reports.effects[&id] else {
                    panic!()
                };
                assert!(binary.operation && binary.result && binary.plan.equality);
                for (step, projected) in binary.projected.into_iter().enumerate() {
                    let port = Port::Projection { point: id, step };
                    if !projected {
                        assert!(!reports.slot_uses.contains_key(&port));
                        continue;
                    }
                    let (owner, slot) = reports.slot_uses[&port];
                    assert_eq!((owner, slot.index), (op.owner, 0));
                    let BinaryClass::SharedScalar(kind) = binary.types.inputs[step] else {
                        panic!()
                    };
                    let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                        panic!()
                    };
                    assert_eq!(slots[0].shape, Shape::SharedScalar(kind));
                    let result = &reports.results[&slot.block].1;
                    assert!(result.dispatch.is_some() && result.consumer.is_none());
                }
            }
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
pub(crate) fn dispatch_reference_binaries_keep_projection_and_source_stages_independent() {
    for step in [0, 1] {
        let source = if step == 0 {
            "n:1;p:&n;x:p.{->$;->tag:true}==p"
        } else {
            "n:1;p:&n;x:p==p.{->$;->tag:true}"
        };
        let (mut checker, mut reports) = checked(source);
        let id = *checker.binaries.keys().next().unwrap();
        let dispatch = *checker.dispatch_ops.keys().next().unwrap();
        let port = Port::Projection { point: id, step };
        let expected = reports.slot_uses[&port];
        for initialized in [false, true] {
            for result in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.initialized = initialized;
                op.result = result;
                for projected in [false, true] {
                    for stage in 0..3 {
                        if !projected && stage == 0 {
                            continue;
                        }
                        let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                            panic!()
                        };
                        op.projected[step] = projected;
                        op.operation = stage == 1;
                        op.result = stage == 2;
                        if stage == 0 {
                            reports.index.operations.remove(&id);
                        } else {
                            reports.index.operations.insert(id, 0);
                        }
                        let actual = checker.slot_uses(&reports, Span::default()).unwrap();
                        assert_eq!(
                            actual.get(&port),
                            (projected && result).then_some(&expected)
                        );
                    }
                }
            }
        }
    }
}

#[test]
pub(crate) fn dispatch_reference_binaries_keep_stopped_order_and_multiple_histories() {
    let (mut checker, reports) =
        checked("d:@\"debug\";n:1;p:&n;x:p.{->$;->tag:true}==d.panic(\"stop\")");
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    assert_eq!(op.plan.normal, [true, true]);
    let (_, Effect::Binary(binary)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(binary.projected, [true, false]);
    assert!(!binary.operation && !binary.result);
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(
        reports
            .slot_uses
            .contains_key(&Port::Projection { point: id, step: 0 })
    );
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    for source in [
        "d:@\"debug\";n:1;p:&n;x:d.panic(\"stop\")==p.{->$;->tag:true}",
        "d:@\"debug\";n:1;p:&n;x:p==p.{->$;->tag:true;d.panic(\"stop\")}",
        "n:1;p:&n;r:p.{->$;->tag:true};x:r==p",
        "n:[1];p:&n;x:p.{->$;->tag:true}==p",
        "n:1;p:&n;x:p.{->$;->tag:true}==p.{->$;->tag:false}",
        "n:1;p:&n;x:p.{->$;->tag:true}.{->$==p}",
        "n:1;p:&n;x:(p.{->$;->tag:true}~<{-><&int32>;tag<boolean>}>)==p",
        "n:1;p:&n;x<&int32>:(p.{->$;->tag:true}~<{-><&int32>;tag<boolean>}>)",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (_, reports) =
        checked("flag:=false;n:1;m:2;p:&n;q:&m;x:p.{|flag|->$;|!flag|->q;->tag:true}==p");
    let slot = reports.slot_uses.values().next().unwrap().1;
    assert_eq!(
        reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
        Sources::Unknown
    );
}
