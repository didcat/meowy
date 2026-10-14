use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn receiver_copy_compositions_retain_typed_sources_and_independent_field_links() {
    for input in ["$", "copy", "alias"] {
        for init in ["{->p;->z:9;->a:true}", "p.{->$;->z:9;->a:true}"] {
            let source = format!(
                "n:1;p:&n;r:{init};flag:=true;out:r.{{copy:$;alias:((copy));joined:{{->({input}~<{{-><&int32>;a<boolean>;z<int32>}}>)}};value:{{->alias.a}};|flag|inner:copy.{{next:$;joined:{{->next}}}}}};f<null>:(p<&int32>){{r:{init};out:r.{{copy:$;alias:copy;joined:{{->{input}}}}}}}"
            );
            let (mut checker, reports) = checked(&source);
            let mut owners = BTreeSet::new();
            let mut linked = 0;
            for (&id, op) in &checker.emissions {
                let Some(composed) = op.composed else {
                    continue;
                };
                assert_eq!(composed.count, 2);
                assert!(composed.shared_primary.is_some());
                let (_, Effect::Emission(observed)) = &reports.effects[&id] else {
                    panic!()
                };
                assert_eq!(observed.composed, Some(composed));
                assert!(observed.initialized.iter().all(|value| *value));
                for (index, target) in op.targets.iter().enumerate() {
                    if let Some(&(owner, slot)) = reports.slot_uses.get(&Port::Emission(target.id))
                    {
                        assert_eq!((owner, slot.index), (op.owner, index));
                        assert_eq!(
                            reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                            Sources::Unknown
                        );
                        linked += 1;
                    }
                }
                owners.insert(op.owner);
            }
            assert_eq!(linked, if input == "$" { 6 } else { 0 });
            assert_eq!(owners, BTreeSet::from([0, 1]));
            assert_eq!(reports.field_results.len(), 1);
            for (&local, row) in &reports.shared_initializers {
                assert!(!reports.eligible.contains(&local));
                assert!(!reports.initializers.contains_key(&local));
                assert!(!reports.receivers.contains_key(&local));
                assert_eq!(row.record.fields, 2);
            }
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
        }
    }
}

#[test]
pub(crate) fn receiver_copy_compositions_keep_stopped_initializers_unobserved() {
    for tail in [
        "d.panic(\"stop\");copy:$;joined:{->copy}",
        "copy<{-><&int32>;a<boolean>;z<int32>}>:{d.panic(\"stop\");->$};joined:{->copy}",
    ] {
        let source = format!("d:@\"debug\";n:1;p:&n;r:{{->p;->z:9;->a:true}};out:r.{{{tail}}}");
        let (mut checker, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty());
        assert!(
            reports
                .candidate_inputs
                .values()
                .all(|(_, input)| input.source.is_none())
        );
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
    }
}
