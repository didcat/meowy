use super::*;

#[test]
pub(crate) fn receiver_copy_coercions_keep_publication_binding_receiver_and_projection_stages_independent()
 {
    for target in ["&int32", "&int32><null"] {
        let source =
            format!("n:1;p:&n;r:p.{{->$;->tag:true}};out:r.{{copy:$;view<{target}>:copy}}");
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
        let local = checker.local_reads[&copy_read(&checker, &reports, id)].local;
        let binding = reports.shared_initializers[&local].init.statement;
        let observed = reports.effects[&binding].clone();
        let expected = reports.slot_uses.clone();
        let convert = checker.coercions[&id].kind == CoercionKind::Convert;
        for flags in 0..32 {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                panic!()
            };
            op.initialized = flags & 1 != 0;
            op.result = flags & 2 != 0;
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                panic!()
            };
            op.initialized = flags & 4 != 0;
            op.result = flags & 8 != 0;
            if flags & 16 != 0 {
                reports.effects.insert(binding, observed.clone());
            } else {
                reports.effects.remove(&binding);
            }
            for projected in [false, true] {
                for operation in [false, true] {
                    if operation && !convert {
                        continue;
                    }
                    for result in [false, true] {
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
                        if !projected && !operation && !result {
                            assert!(
                                checker
                                    .slot_uses(&reports, Span::default())
                                    .unwrap_err()
                                    .message
                                    .contains("identity")
                            );
                            continue;
                        }
                        let ready = flags & 2 != 0 && flags & 4 != 0 && flags & 16 != 0;
                        assert_eq!(
                            checker.slot_uses(&reports, Span::default()).unwrap(),
                            if projected && ready {
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
pub(crate) fn receiver_copy_coercions_preserve_referent_kinds_and_stopped_prefixes() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        let source = format!(
            "d:@\"debug\";n<{ty}>:{value};p:&n;r:{{->p;->tag:true}};out:r.{{copy:$;alias:((copy));a<&{ty}><null>:alias;d.panic(\"stop\");b<&{ty}>:copy}}"
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
    for tail in [
        "d.panic(\"stop\");copy:$;view<&int32>:copy",
        "copy<{-><&int32>;tag<boolean>}>:{d.panic(\"stop\");->$};view<&int32>:copy",
    ] {
        let source = format!("d:@\"debug\";n:1;p:&n;r:{{->p;->tag:true}};out:r.{{{tail}}}");
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty());
    }
}

#[test]
pub(crate) fn receiver_copy_coercions_require_counts_without_changing_direct_receiver_fallback() {
    for (init, input, linked) in [
        ("{->p;->tag:true}", "$", true),
        ("{->p;->tag:true}", "copy", false),
        ("r", "$", false),
        ("r", "copy", false),
    ] {
        let source =
            format!("n:1;p:&n;r:{{->p;->tag:true}};out:({init}).{{copy:$;view<&int32>:({input})}}");
        let (mut checker, mut reports) = checked(&source);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary)
            .unwrap()
            .0;
        let expected = reports.slot_uses.clone();
        checker.coercions.get_mut(&id).unwrap().fields = None;
        let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.fields = None;
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            if linked {
                expected.clone()
            } else {
                Uses::new()
            }
        );
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
        assert_eq!(reports.slot_uses, expected);
    }
}
