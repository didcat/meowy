use super::*;

#[test]
pub(crate) fn shared_receiver_copy_fields_keep_source_binding_receiver_and_field_stages_independent()
 {
    let (mut checker, mut reports) =
        checked("n:1;p:&n;r:p.{->$;->tag:true};out:r.{copy:$;->copy.tag}");
    let id = *checker.fields.keys().next().unwrap();
    let expected = reports.slot_uses[&Port::Operation(id)];
    let source = reports.results[&expected.1.block].1.dispatch.unwrap();
    let receiver = reports
        .receivers
        .values()
        .find(|row| row.2.is_some())
        .unwrap()
        .1;
    let local = checker
        .local_reads
        .iter()
        .find(|(read, op)| {
            checker.points[**read].site == checker.points[id].site
                && reports.shared_initializers.contains_key(&op.local)
        })
        .unwrap()
        .1
        .local;
    let binding = reports.shared_initializers[&local].init.statement;
    let bound = reports.effects[&binding].clone();
    for source_init in [false, true] {
        for published in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                panic!()
            };
            op.initialized = source_init;
            op.result = published;
            for seen in [false, true] {
                if seen {
                    reports.effects.insert(binding, bound.clone());
                } else {
                    reports.effects.remove(&binding);
                }
                for initialized in [false, true] {
                    for completed in [false, true] {
                        let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap()
                        else {
                            panic!()
                        };
                        op.initialized = initialized;
                        op.result = completed;
                        for (operation, result) in [(true, false), (true, true), (false, true)] {
                            let (
                                _,
                                Effect::Field {
                                    operation: observed,
                                    result: done,
                                    ..
                                },
                            ) = reports.effects.get_mut(&id).unwrap()
                            else {
                                panic!()
                            };
                            *observed = operation;
                            *done = result;
                            reports.slot_uses =
                                checker.slot_uses(&reports, Span::default()).unwrap();
                            assert_eq!(
                                reports.slot_uses.get(&Port::Operation(id)),
                                (published && seen && initialized && operation)
                                    .then_some(&expected)
                            );
                            assert_eq!(
                                checker
                                    .field_result_slot(
                                        &reports,
                                        id,
                                        0,
                                        &reports.effects[&id].1,
                                        Span::default()
                                    )
                                    .unwrap(),
                                (published && seen && initialized && operation && result)
                                    .then_some(expected.1)
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
pub(crate) fn shared_receiver_copy_fields_preserve_referent_kinds_and_stopped_initialization() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        let source = format!(
            "d:@\"debug\";n<{ty}>:{value};p:&n;r:{{->p;->tag:true}};out:r.{{copy:$;a:copy.tag;d.panic(\"stop\");b:copy.tag}}"
        );
        let (checker, reports) = checked(&source);
        assert_eq!(checker.fields.len(), 2);
        assert_eq!(reports.slot_uses.len(), 1);
        assert_eq!(reports.field_results.len(), 1);
        let (&id, op) = checker.fields.first_key_value().unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
            panic!()
        };
        assert_eq!(
            layout[0].shape,
            Shape::SharedScalar(op.shared_primary.unwrap())
        );
        assert_eq!(
            reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
            Sources::Unknown
        );
    }
    for tail in [
        "d.panic(\"stop\");copy:$;a:copy.tag",
        "copy<{-><&int32>;tag<boolean>}>:{d.panic(\"stop\");->$};a:copy.tag",
    ] {
        let source = format!("d:@\"debug\";n:1;p:&n;r:{{->p;->tag:true}};out:r.{{{tail}}}");
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty());
        assert!(reports.field_results.is_empty());
    }
}

#[test]
pub(crate) fn shared_receiver_copy_fields_keep_mutable_load_parameter_and_nonscalar_sources_opaque()
{
    for source in [
        "n:1;p:&n;r:{->p;->tag:true};out:r.{copy:=$;->copy.tag}",
        "n:1;p:&n;r:={->p;->tag:true};out:r.{copy:$;->copy.tag}",
        "n:1;p:&n;r:{->p;->tag:=true};out:r.{copy:$;->copy.tag}",
        "n:1;p:&n;r:{->p;->other:p;->tag:true};out:r.{copy:$;->copy.tag}",
        "n:1;p:&n;r:{->p;->tag:true};out:(&r).{copy:$;->copy.tag}",
        "n:1;p:&n;r:{->p;->inner:{->n:1}};out:r.{copy:$;->copy.inner}",
        "f<boolean>:(r<{-><&int32>;tag<boolean>}>){->r.{copy:$;->copy.tag}}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
        assert!(reports.field_results.is_empty(), "{source}");
    }
}
