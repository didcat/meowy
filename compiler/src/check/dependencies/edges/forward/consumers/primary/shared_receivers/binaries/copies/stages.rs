use super::*;

#[test]
pub(crate) fn receiver_copy_binaries_keep_publication_binding_receiver_and_operand_stages_independent()
 {
    for (tail, step) in [("copy==p", 0), ("p!=copy", 1)] {
        let source = format!("n:1;p:&n;r:p.{{->$;->tag:true}};out:r.{{copy:$;->{tail}}}");
        let (mut checker, mut reports) = checked(&source);
        let id = *checker.binaries.keys().next().unwrap();
        let port = Port::Projection { point: id, step };
        let block = reports.slot_uses[&port].1.block;
        let source = reports.results[&block].1.dispatch.unwrap();
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
                    for result in [false, true] {
                        let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                            panic!()
                        };
                        op.projected[step] = projected;
                        op.operation = operation;
                        op.result = result;
                        if !operation && !result {
                            reports.index.operations.remove(&id);
                        } else {
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
        reports.index.operations.remove(&id);
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(reports.slot_uses, expected);
    }
}

#[test]
pub(crate) fn receiver_copy_binaries_require_each_operand_count_and_preserve_direct_fallback() {
    for (init, input, linked) in [
        ("{->p;->tag:true}", "$", true),
        ("{->p;->tag:true}", "copy", false),
        ("r", "$", false),
        ("r", "copy", false),
    ] {
        for step in 0..2 {
            let tail = if step == 0 {
                format!("{input}==p")
            } else {
                format!("p!={input}")
            };
            let source = format!("n:1;p:&n;r:{{->p;->tag:true}};out:({init}).{{copy:$;->{tail}}}");
            let (mut checker, mut reports) = checked(&source);
            let id = *checker.binaries.keys().next().unwrap();
            let expected = reports.slot_uses.clone();
            assert_eq!(expected.len(), 1);
            assert_eq!(checker.binaries[&id].fields[step], Some(1));
            checker.binaries.get_mut(&id).unwrap().fields[step] = None;
            let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.fields[step] = None;
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                if linked {
                    expected.clone()
                } else {
                    Uses::new()
                }
            );
            checker.binaries.get_mut(&id).unwrap().fields[step] = Some(2);
            let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.fields[step] = Some(2);
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
}

#[test]
pub(crate) fn receiver_copy_binaries_preserve_referents_and_stopped_copies_on_both_sides() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        for tail in [
            "alias==p",
            "p!=alias",
            "alias==d.panic(\"stop\")",
            "d.panic(\"stop\")==alias",
        ] {
            let source = format!(
                "d:@\"debug\";n<{ty}>:{value};p:&n;r:{{->p;->tag:true}};out:r.{{copy:$;alias:((copy));->{tail}}}"
            );
            let (checker, reports) = checked(&source);
            let (&id, op) = checker.binaries.first_key_value().unwrap();
            if tail.starts_with('d') {
                assert!(!reports.effects.contains_key(&id));
                assert!(reports.slot_uses.is_empty());
                continue;
            }
            let step = usize::from(tail.starts_with('p'));
            let slot = reports.slot_uses[&Port::Projection { point: id, step }].1;
            let BinaryClass::SharedScalar(kind) = op.types.inputs[step] else {
                panic!()
            };
            let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(layout[0].shape, Shape::SharedScalar(kind));
            assert_eq!(op.fields[step], Some(1));
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
            let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(
                (observed.operation, observed.result),
                (!tail.contains("panic"), !tail.contains("panic"))
            );
        }
    }
    for tail in [
        "d.panic(\"stop\");copy:$;->copy==p",
        "copy<{-><&int32>;tag<boolean>}>:{d.panic(\"stop\");->$};->p==copy",
    ] {
        let source = format!("d:@\"debug\";n:1;p:&n;r:{{->p;->tag:true}};out:r.{{{tail}}}");
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty());
    }
}
