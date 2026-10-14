use super::*;

#[test]
pub(crate) fn shared_local_receiver_coercions_keep_source_receiver_and_conversion_stages_independent()
 {
    for target in ["&int32", "&int32><null"] {
        let source = format!("n:1;p:&n;r:p.{{->$;->tag:true}};out:r.{{copy<{target}>:$}}");
        let (mut checker, mut reports) = checked(&source);
        let (&port, &(_, slot)) = reports.slot_uses.first_key_value().unwrap();
        let Port::Projection { point: id, step: 0 } = port else {
            panic!()
        };
        let source = reports.results[&slot.block].1.dispatch.unwrap();
        let receiver = reports
            .receivers
            .values()
            .find(|row| row.2.is_some())
            .unwrap()
            .1;
        let expected = reports.slot_uses.clone();
        let convert = checker.coercions[&id].kind == CoercionKind::Convert;
        for source_init in [false, true] {
            for published in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.initialized = source_init;
                op.result = published;
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
pub(crate) fn shared_local_receiver_coercions_preserve_referent_kinds_and_stopped_prefixes() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        let source = format!(
            "d:@\"debug\";n<{ty}>:{value};p:&n;r:{{->p;->tag:true}};out:r.{{a<&{ty}><null>:$;d.panic(\"stop\");b<&{ty}>:$}}"
        );
        let (checker, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 1);
        let ops: Vec<_> = checker
            .coercions
            .iter()
            .filter(|(_, op)| op.primary)
            .collect();
        assert_eq!(ops.len(), 2);
        assert_eq!(
            ops.iter()
                .filter(|(id, _)| reports.effects.contains_key(id))
                .count(),
            1
        );
        let (&id, op) = ops[0];
        let slot = reports.slot_uses[&Port::Projection { point: id, step: 0 }].1;
        let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
            panic!()
        };
        assert_eq!(op.source, Some(layout[0].shape));
        assert_eq!(op.fields, Some(1));
        assert_eq!(
            reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
            Sources::Unknown
        );
    }
    for source in [
        "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true;d.panic(\"stop\")};out:r.{a<&int32>:$}",
        "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true};out:r.{d.panic(\"stop\");a<&int32>:$}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty());
    }
}

#[test]
pub(crate) fn shared_local_receiver_coercions_require_counts_without_changing_direct_receiver_fallback()
 {
    for stored in [false, true] {
        let source = if stored {
            "n:1;r:{->&n;->tag:true};out:r.{copy<&int32>:$}"
        } else {
            "n:1;out:{->&n;->tag:true}.{copy<&int32>:$}"
        };
        let (mut checker, mut reports) = checked(source);
        let (&id, _) = checker.coercions.iter().find(|(_, op)| op.primary).unwrap();
        let expected = reports.slot_uses.clone();
        checker.coercions.get_mut(&id).unwrap().fields = None;
        let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.fields = None;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            if stored { Uses::new() } else { expected }
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        checker.coercions.get_mut(&id).unwrap().fields = Some(2);
        let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.fields = Some(2);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
