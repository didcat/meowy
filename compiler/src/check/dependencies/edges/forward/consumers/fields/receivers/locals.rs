use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

mod boundaries;
mod evidence;
mod limits;

#[test]
pub(crate) fn shared_local_receiver_fields_keep_source_receiver_and_field_stages_independent() {
    let (mut checker, mut reports) =
        checked("n:1;p:&n;r:p.{->$;->tag:true};out:r.{copy:{->$.tag}}");
    let id = *checker.fields.keys().next().unwrap();
    let expected = reports.slot_uses[&Port::Operation(id)];
    let source = reports.results[&expected.1.block].1.dispatch.unwrap();
    let receiver = reports
        .receivers
        .values()
        .find(|row| row.2.is_some())
        .unwrap()
        .1;
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
                    for (operation, result) in [(true, false), (true, true), (false, true)] {
                        let (
                            _,
                            Effect::Field {
                                operation: seen,
                                result: done,
                                ..
                            },
                        ) = reports.effects.get_mut(&id).unwrap()
                        else {
                            panic!()
                        };
                        *seen = operation;
                        *done = result;
                        reports.slot_uses = checker.slot_uses(&reports, Span::default()).unwrap();
                        assert_eq!(
                            reports.slot_uses.get(&Port::Operation(id)),
                            (published && initialized && operation).then_some(&expected)
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
                            (published && initialized && operation && result).then_some(expected.1)
                        );
                    }
                }
            }
        }
    }
}

#[test]
pub(crate) fn shared_local_receiver_fields_preserve_scalar_kinds_and_stopped_prefixes() {
    for binding in [
        "n<int8>:1",
        "n<uint64>:1",
        "n:true",
        "n<float32>:1.5",
        "n:\"cat\"",
    ] {
        let source = format!(
            "d:@\"debug\";{binding};p:&n;r:{{->p;->tag:true;->text:\"cat\"}};out:r.{{a:$.text;d.panic(\"stop\");b:$.tag}}"
        );
        let (checker, reports) = checked(&source);
        assert_eq!(checker.fields.len(), 2);
        assert_eq!(reports.slot_uses.len(), 1);
        assert_eq!(reports.field_results.len(), 1);
        let (&first, _) = checker.fields.first_key_value().unwrap();
        let slot = reports.slot_uses[&Port::Operation(first)].1;
        assert_eq!(
            reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
            Sources::Unknown
        );
    }
}

#[test]
pub(crate) fn shared_local_receiver_fields_link_stored_inputs_after_receiver_qualification() {
    for init in ["{->p;->tag:true;->n:3}", "p.{->$;->tag:true;->n:3}"] {
        let source = format!(
            "n:1;p:&n;r:{init};copy:((r));flag:=true;out:copy.{{a:{{->(($)).tag}};|flag|inner:$.{{->$.n}};->$.tag}};f<boolean>:(p<&int32>){{r:{init};->r.{{->$.tag}}}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(checker.fields.len(), 4);
        assert_eq!(reports.slot_uses.len(), 4);
        assert_eq!(reports.field_results.len(), 4);
        for (&id, op) in &checker.fields {
            let (owner, slot) = reports.slot_uses[&Port::Operation(id)];
            assert_eq!((owner, slot.index), (op.owner, op.index + 1));
            assert_eq!(reports.field_results[&Port::Normal(id)], (owner, slot));
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
            assert!(reports.direct_sources.values().any(|(_, direct)| {
                direct
                    .source
                    .is_some_and(|source| source.field == id && source.slot == slot)
            }));
        }
        let mut owners = BTreeSet::new();
        for (&local, &(owner, dispatch, kind)) in &reports.receivers {
            let Some(kind) = kind else { continue };
            assert!(!reports.eligible.contains(&local));
            assert!(!reports.shared_initializers.contains_key(&local));
            let reads: Vec<_> = checker
                .local_reads
                .iter()
                .filter(|(_, read)| read.local == local)
                .map(|(&id, _)| id)
                .collect();
            assert!(!reads.is_empty());
            for id in reads {
                assert_eq!(
                    checker
                        .shared_receiver_input(&reports, id, owner, kind, Span::default())
                        .unwrap(),
                    Some(checker.dispatch_ops[&dispatch].input)
                );
                assert_eq!(
                    checker
                        .shared_receiver_body(&reports, id, owner, kind, Span::default())
                        .unwrap(),
                    None
                );
            }
            owners.insert(owner);
        }
        assert_eq!(owners, BTreeSet::from([0, 1]));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(
            checker.field_results(&reports, Span::default()).unwrap().0,
            reports.field_results
        );
        assert_eq!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.expanded_walk
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_receiver_fields_keep_mutable_parameter_and_aggregate_inputs_opaque() {
    for source in [
        "n:1;p:&n;r:={->p;->tag:true};out:r.{->$.tag}",
        "n:1;p:&n;r:{->p;->tag:=true};out:r.{->$.tag}",
        "n:1;p:&n;r:{->p;->other:p;->tag:true};out:r.{->$.tag}",
        "n:1;p:&n;r:{->p;->tag:true};out:(&r).{->$.tag}",
        "n:1;p:&n;r:{->p;->inner:{->n:1}};out:r.{->$.inner}",
        "f<boolean>:(r<{-><&int32>;tag<boolean>}>){->r.{->$.tag}}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
        assert!(reports.field_results.is_empty(), "{source}");
    }
}
