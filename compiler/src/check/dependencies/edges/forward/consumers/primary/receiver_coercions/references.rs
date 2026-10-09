use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn shared_receiver_coercions_link_exact_slots_through_contexts_scopes_and_owners() {
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
                "n<{ty}>:{value};p:&n;flag:=true;out:({init}).{{a<&{ty}>:(($));b<&{ty}><null>:$;|flag|inner:$.{{c<&{ty}>:($~<{{-><&{ty}>;tag<boolean>}}> )}};cmp:p!=(($))}};f<&{ty}>:(p<&{ty}>){{->({init}).{{->$}}}}"
            );
            let (mut checker, reports) = checked(&source);
            assert_eq!(reports.slot_uses.len(), 5, "{source}");
            let mut owners = BTreeSet::new();
            let mut converted = false;
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            for (&port, &(owner, slot)) in &reports.slot_uses {
                let Port::Projection { point, step: 0 } = port else {
                    panic!()
                };
                let (_, Effect::Coercion(op)) = &reports.effects[&point] else {
                    panic!()
                };
                assert!(op.primary && op.projected && op.result);
                assert_eq!(op.operation, op.op == CoercionKind::Convert);
                assert!(matches!(op.source, Some(Shape::SharedScalar(_))));
                let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(op.source, Some(slots[0].shape));
                assert_eq!((slot.index, checker.bodies[&slot.block].owner), (0, owner));
                assert_eq!(
                    reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                    Sources::Unknown
                );
                assert_eq!(
                    checker
                        .forward_coercion_input(&reports, point, owner, Span::default())
                        .unwrap(),
                    None
                );
                owners.insert(owner);
                converted |= op.op == CoercionKind::Convert;
            }
            assert_eq!(owners, BTreeSet::from([0, 1]));
            assert!(converted);
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
}

#[test]
pub(crate) fn shared_receiver_coercions_keep_source_receiver_and_consumer_stages_independent() {
    for target in ["&int32", "&int32><null"] {
        let source = format!("n:1;p:&n;out:p.{{->$;->tag:true}}.{{copy<{target}>:$}}");
        let (mut checker, mut reports) = checked(&source);
        let (&port, &(_, slot)) = reports.slot_uses.first_key_value().unwrap();
        let Port::Projection { point: id, step: 0 } = port else {
            panic!()
        };
        let source = reports.results[&slot.block].1.dispatch.unwrap();
        let receiver = *checker
            .dispatch_ops
            .iter()
            .find(|(_, op)| op.shared_primary.is_some())
            .unwrap()
            .0;
        let expected = reports.slot_uses.clone();
        let convert = checker.coercions[&id].kind == CoercionKind::Convert;
        for published in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                panic!()
            };
            op.result = published;
            op.initialized = !published;
            for initialized in [false, true] {
                for completed in [false, true] {
                    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap()
                    else {
                        panic!()
                    };
                    op.initialized = initialized;
                    op.result = completed;
                    for (projected, operation, result) in [
                        (true, false, false),
                        (true, convert, true),
                        (false, convert, true),
                        (false, false, true),
                    ] {
                        let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap()
                        else {
                            panic!()
                        };
                        op.projected = projected;
                        op.operation = operation;
                        op.result = result;
                        if !operation && !result {
                            reports.index.operations.remove(&id);
                        } else if convert {
                            reports.index.operations.insert(id, 0);
                        }
                        assert_eq!(
                            checker.slot_uses(&reports, Span::default()).unwrap(),
                            if projected && initialized && published {
                                expected.clone()
                            } else {
                                Uses::new()
                            }
                        );
                    }
                }
            }
        }
        if convert {
            reports.index.operations.remove(&id);
            assert!(
                checker
                    .slot_uses(&reports, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
        }
        assert_eq!(reports.slot_uses, expected);
    }
}

#[test]
pub(crate) fn shared_receiver_coercions_keep_stopped_prefixes_and_opaque_storage_boundaries() {
    let (mut checker, reports) = checked(
        "d:@\"debug\";n:1;p:&n;out:p.{->$;->tag:true}.{a<&int32>:$;d.panic(\"stop\");b<&int32>:$}",
    );
    assert_eq!(reports.slot_uses.len(), 1);
    assert_eq!(
        checker
            .coercions
            .values()
            .filter(|op| matches!(op.source, Some(Shape::SharedScalar(_))))
            .count(),
        2
    );
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    for source in [
        "n:1;p:&n;r:{->p;->tag:true};x:r.{copy<&int32>:$}",
        "n:1;p:&n;x:{->p;->tag:=true}.{copy<&int32>:$}",
        "n:1;p:&n;x:{->p;->other:p}.{copy<&int32>:$}",
        "n:1;p:&n;x:p.{->$;->tag:true}.{copy:$;v<&int32>:copy}",
        "n:1;p:&n;r:{->p;->tag:true};x:(&r).{copy<&int32>:*$}",
        "n:[1];p:&n;x:{->p;->tag:true}.{copy<&int32[1]>:$}",
        "d:@\"debug\";n:1;p:&n;x:p.{->$;->tag:true}.{d.panic(\"stop\");copy<&int32>:$}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
