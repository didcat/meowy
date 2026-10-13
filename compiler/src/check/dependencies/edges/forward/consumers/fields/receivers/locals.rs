use super::*;

#[test]
pub(crate) fn shared_local_receiver_fields_stop_at_stored_inputs_after_receiver_qualification() {
    for init in ["{->p;->tag:true;->n:3}", "p.{->$;->tag:true;->n:3}"] {
        let source = format!(
            "n:1;p:&n;r:{init};copy:((r));flag:=true;out:copy.{{a:{{->(($)).tag}};|flag|inner:$.{{->$.n}};->$.tag}};f<boolean>:(p<&int32>){{r:{init};->r.{{->$.tag}}}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(checker.fields.len(), 4);
        assert!(reports.slot_uses.is_empty());
        assert!(reports.field_results.is_empty());
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
    }
}

#[test]
pub(crate) fn shared_local_receiver_fields_keep_mutable_parameter_and_copy_inputs_opaque() {
    for source in [
        "n:1;p:&n;r:={->p;->tag:true};out:r.{->$.tag}",
        "n:1;p:&n;r:{->p;->tag:=true};out:r.{->$.tag}",
        "n:1;p:&n;r:{->p;->other:p;->tag:true};out:r.{->$.tag}",
        "n:1;p:&n;r:{->p;->tag:true};out:(&r).{->$.tag}",
        "n:1;p:&n;r:{->p;->tag:true};out:r.{copy:$;->copy.tag}",
        "n:1;p:&n;r:{->p;->inner:{->n:1}};out:r.{->$.inner}",
        "f<boolean>:(r<{-><&int32>;tag<boolean>}>){->r.{->$.tag}}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
        assert!(reports.field_results.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn shared_local_receiver_consumers_keep_primary_and_composition_paths_separate() {
    let (checker, reports) =
        checked("n:1;p:&n;r:{->p;->tag:true};out:r.{a:$==p;b<&int32>:$;copy:{->$};->true}");
    assert!(reports.slot_uses.is_empty());
    assert!(checker.binaries.values().any(|op| op.plan.primary[0]));
    assert!(checker.coercions.values().any(|op| op.primary));
    assert!(checker.emissions.values().any(|op| op.composed.is_some()));
}
