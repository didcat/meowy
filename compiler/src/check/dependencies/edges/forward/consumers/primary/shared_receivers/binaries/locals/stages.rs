use super::*;

#[test]
pub(crate) fn shared_local_receiver_binaries_keep_source_receiver_and_projection_stages_independent()
 {
    let (mut checker, mut reports) = checked("n:1;p:&n;r:p.{->$;->tag:true};out:r.{->$==p}");
    let id = *checker.binaries.keys().next().unwrap();
    let port = Port::Projection { point: id, step: 0 };
    let block = reports.slot_uses[&port].1.block;
    let source = reports.results[&block].1.dispatch.unwrap();
    let receiver = reports
        .receivers
        .values()
        .find(|row| row.2.is_some())
        .unwrap()
        .1;
    let expected = reports.slot_uses.clone();
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
                            if published && initialized && projected {
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

#[test]
pub(crate) fn shared_local_receiver_binaries_preserve_referents_and_stopped_operand_order() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        for tail in ["$==p", "$==d.panic(\"stop\")", "d.panic(\"stop\")==$"] {
            let source = format!(
                "d:@\"debug\";n<{ty}>:{value};p:&n;r:{{->p;->tag:true}};out:r.{{->{tail}}}"
            );
            let (checker, reports) = checked(&source);
            let (&id, op) = checker.binaries.first_key_value().unwrap();
            if tail.starts_with('d') {
                assert!(!reports.effects.contains_key(&id));
                assert!(reports.slot_uses.is_empty());
                assert!(
                    !reports.entries[&0]
                        .1
                        .ports
                        .contains(&Port::Entry(op.inputs[1]))
                );
                continue;
            }
            let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(op.plan.primary, [true, false]);
            assert_eq!(op.plan.normal, [true, true]);
            assert_eq!(observed.projected, [true, false]);
            assert_eq!(
                (observed.operation, observed.result),
                (tail == "$==p", tail == "$==p")
            );
            let slot = reports.slot_uses[&Port::Projection { point: id, step: 0 }].1;
            let BinaryClass::SharedScalar(kind) = op.types.inputs[0] else {
                panic!()
            };
            let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(layout[0].shape, Shape::SharedScalar(kind));
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
        }
    }
}

#[test]
pub(crate) fn shared_local_receiver_binaries_require_counts_without_changing_direct_fallback() {
    for stored in [false, true] {
        let source = if stored {
            "n:1;r:{->&n;->tag:true};out:r.{->$==&n}"
        } else {
            "n:1;out:{->&n;->tag:true}.{->$==&n}"
        };
        let (mut checker, mut reports) = checked(source);
        let id = *checker.binaries.keys().next().unwrap();
        let expected = reports.slot_uses.clone();
        checker.binaries.get_mut(&id).unwrap().fields = [None; 2];
        let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.fields = [None; 2];
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            if stored { Uses::new() } else { expected }
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        checker.binaries.get_mut(&id).unwrap().fields[0] = Some(2);
        let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.fields[0] = Some(2);
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
