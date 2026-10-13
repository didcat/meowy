use super::{super::super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn shared_local_fields_follow_original_block_dispatch_and_copy_sources() {
    for init in ["{->p;->tag:true;->n:3}", "p.{->$;->tag:true;->n:3}"] {
        let source = format!(
            "n:1;p:&n;r:{init};copy:((r));out:{{->((copy)~<{{-><&int32>;tag<boolean>;n<int32>}}>).tag}};f<boolean>:(p<&int32>,flag<boolean>){{r:{init};|flag|{{copy:r;out:copy.n}};->r.tag}}"
        );
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 3);
        for (&id, op) in &checker.fields {
            let (owner, slot) = reports.slot_uses[&Port::Operation(id)];
            assert_eq!(owner, op.owner);
            assert_eq!(slot.index, op.index + 1);
            assert_eq!(reports.field_results[&Port::Normal(id)], (owner, slot));
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
        }
        assert!(reports.slot_uses.values().any(|(owner, _)| *owner != 0));
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
    }
}

#[test]
pub(crate) fn shared_local_fields_keep_alias_load_receiver_copy_and_other_consumers_opaque() {
    for tail in [
        "out:r.{copy:$;->copy.tag}",
        "out:{->named:r;copy:named.tag}",
        "out:(&r).tag",
        "out:(*(&r)).tag",
        "out:r==p",
        "out:{->r}",
    ] {
        let source = format!("n:1;p:&n;r:{{->p;->tag:true}};{tail}");
        crate::compile(&source).unwrap();
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
