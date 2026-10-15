use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

mod boundaries;
mod evidence;
mod limits;
mod stages;

#[test]
pub(crate) fn shared_receiver_copies_retain_typed_bindings_and_original_receiver_reads() {
    let (mut checker, reports) = checked("n:1;p:&n;r:{->p;->tag:true};out:r.{copy:$;->copy.tag}");
    let (&id, op) = checker.fields.first_key_value().unwrap();
    let input = op.input;
    let (&read, op) = checker
        .local_reads
        .iter()
        .find(|(read, op)| {
            checker.points[**read].site == checker.points[id].site
                && reports.shared_initializers.contains_key(&op.local)
        })
        .unwrap();
    let local = op.local;
    let row = reports.shared_initializers[&local];
    assert!(!reports.eligible.contains(&local));
    assert!(!reports.initializers.contains_key(&local));
    assert_eq!(
        checker
            .read_shared_initializer_input(&reports, read, 0, row.record, Span::default())
            .unwrap(),
        row.init.input
    );
    assert!(row.init.input.is_some());
    let (&cell, &(_, receiver, kind)) = reports.receivers.first_key_value().unwrap();
    assert_eq!(kind, Some(row.record.primary));
    assert!(!reports.shared_initializers.contains_key(&cell));
    assert!(!reports.eligible.contains(&cell));
    let original = *checker
        .local_reads
        .iter()
        .find(|(id, op)| {
            op.local == cell && checker.points[**id].site == checker.points[row.init.statement].site
        })
        .unwrap()
        .0;
    assert_eq!(
        checker
            .shared_receiver_source(&reports, original, 0, row.record.primary, Span::default())
            .unwrap(),
        Some((checker.dispatch_ops[&receiver].input, row.record))
    );
    assert_eq!(
        checker
            .shared_local_body(&reports, input, 0, row.record, Span::default())
            .unwrap(),
        None
    );
    assert_eq!(
        checker
            .shared_record_receiver_body(&reports, input, 0, row.record, Span::default())
            .unwrap(),
        None
    );
    assert_eq!(reports.slot_uses.len(), 1);
}

#[test]
pub(crate) fn shared_receiver_copy_fields_link_original_slots_across_scopes_and_owners() {
    for init in ["{->p;->tag:true;->n:3}", "p.{->$;->tag:true;->n:3}"] {
        let source = format!(
            "n:1;p:&n;r:{init};flag:=true;out:r.{{copy:$;alias:((copy)~<{{-><&int32>;tag<boolean>;n<int32>}}>);direct:{{->$.tag}};a:{{->alias.tag}};|flag|inner:copy.{{next:$;->next.n}};->copy.tag}};f<boolean>:(p<&int32>){{r:{init};->r.{{copy:$;->copy.tag}}}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(checker.fields.len(), 5);
        assert_eq!(reports.slot_uses.len(), 5);
        assert_eq!(reports.field_results.len(), 5);
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
        assert!(checker.fields.values().any(|op| op.owner != 0));
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
    }
}

#[test]
pub(crate) fn shared_receiver_copies_keep_binary_owned_primaries_opaque() {
    let (checker, reports) =
        checked("n:1;p:&n;r:{->p;->tag:true};out:r.{copy:$;cmp:copy==p;nested:copy.{a:$==p}}");
    assert!(checker.binaries.values().any(|op| op.plan.primary[0]));
    assert!(checker.coercions.values().all(|op| !op.primary));
    assert!(reports.slot_uses.is_empty());
}
