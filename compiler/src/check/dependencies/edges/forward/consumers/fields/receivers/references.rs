use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn shared_receiver_fields_link_named_slots_results_and_candidate_consumers() {
    for init in ["{->p;->z:9;->a:true}", "p.{->$;->z:9;->a:true}"] {
        let source = format!(
            "n:1;p:&n;flag:=true;out:({init}).{{copy:{{->(($)).a}};|flag|inner:$.{{->$.z}};->$.a}};f<uint8>:(p<&int32>){{v<uint8>:7;->{{->p;->a:v}}.{{->$.a}}}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(checker.fields.len(), 4);
        assert_eq!(reports.slot_uses.len(), 4);
        assert_eq!(reports.field_results.len(), 4);
        let mut owners = BTreeSet::new();
        for (&id, op) in &checker.fields {
            let (owner, slot) = reports.slot_uses[&Port::Operation(id)];
            assert_eq!((owner, slot.index), (op.owner, op.index + 1));
            assert_eq!(reports.field_results[&Port::Normal(id)], (owner, slot));
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(
                slots[0].shape,
                Shape::SharedScalar(op.shared_primary.unwrap())
            );
            assert!(matches!(slots[slot.index].shape, Shape::Scalar(_)));
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
            assert!(reports.direct_sources.values().any(|(_, direct)| {
                direct
                    .source
                    .is_some_and(|source| source.field == id && source.slot == slot)
            }));
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
pub(crate) fn shared_receiver_fields_keep_source_receiver_operation_and_result_visits_independent()
{
    let (mut checker, mut reports) = checked("n:1;p:&n;out:p.{->$;->tag:true}.{->$.tag}");
    let id = *checker.fields.keys().next().unwrap();
    let expected = reports.slot_uses[&Port::Operation(id)];
    let source = reports.results[&expected.1.block].1.dispatch.unwrap();
    let receiver = *checker
        .dispatch_ops
        .iter()
        .find(|(_, op)| op.shared_primary.is_some())
        .unwrap()
        .0;
    for published in [false, true] {
        let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
            panic!()
        };
        op.result = published;
        op.initialized = !published;
        for initialized in [false, true] {
            for completed in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
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
                    let results = checker.field_results(&reports, Span::default()).unwrap().0;
                    assert_eq!(
                        results.get(&Port::Normal(id)),
                        (published && initialized && operation && result).then_some(&expected)
                    );
                }
            }
        }
    }
}

#[test]
pub(crate) fn shared_receiver_fields_preserve_stopped_prefixes_and_ineligible_paths() {
    let (checker, reports) =
        checked("d:@\"debug\";n:1;p:&n;out:p.{->$;->tag:true}.{a:$.tag;d.panic(\"stop\");b:$.tag}");
    assert_eq!(checker.fields.len(), 2);
    assert_eq!(reports.slot_uses.len(), 1);
    assert_eq!(reports.field_results.len(), 1);
    for source in [
        "n:1;p:&n;r:{->p;->tag:true};out:r.{->$.tag}",
        "n:1;p:&n;out:{->p;->tag:=true}.{->$.tag}",
        "n:1;p:&n;out:{->p;->other:p;->tag:true}.{->$.tag}",
        "n:1;p:&n;r:{->p;->tag:true};out:(&r).{->$.tag}",
        "n:1;p:&n;r:{->p;->tag:true};out:(&r).{->(*$).tag}",
        "n:1;p:&n;out:{->p;->inner:{->n:1}}.{->$.inner}",
        "n:[1];p:&n;out:{->p;->tag:true}.{->$.tag}",
        "n:1;p:&n;out:p.{->$;->tag:true}.{copy:$;->copy.tag}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
        assert!(reports.field_results.is_empty(), "{source}");
    }
}
