use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

mod boundaries;
mod evidence;
mod limits;

pub(super) const SOURCE: &str =
    "n:1;p:&n;r:p.{->$;->z:9;->a:true};out:r.{copy:(($));alias:((copy));joined:{->alias}}";

pub(super) fn copy_read(checker: &Checker, reports: &Reports, id: PointId) -> PointId {
    *checker
        .local_reads
        .iter()
        .find(|(read, op)| {
            checker.points[**read].site == checker.points[id].site
                && reports.shared_initializers.contains_key(&op.local)
        })
        .unwrap()
        .0
}

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
            assert_eq!(linked, 9);
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

#[test]
pub(crate) fn receiver_copy_compositions_keep_source_binding_receiver_and_target_stages_independent()
 {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
    let targets = op.targets.clone();
    let expected = reports.slot_uses.clone();
    let slot = expected[&Port::Emission(targets[0].id)].1;
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
    reports.blocks.remove(&targets[0].block);
    reports.results.remove(&targets[0].block);
    for flags in 0..64 {
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
        for selected in 0..8 {
            let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.initialized = (0..3).map(|index| selected & (1 << index) != 0).collect();
            op.result = flags & 32 != 0;
            let wanted = targets
                .iter()
                .enumerate()
                .filter_map(|(index, target)| {
                    let port = Port::Emission(target.id);
                    (flags & 22 == 22 && selected & (1 << index) != 0)
                        .then_some((port, expected[&port]))
                })
                .collect::<Uses>();
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                wanted,
                "{flags}, {selected}"
            );
        }
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn receiver_copy_compositions_preserve_scalar_kinds_and_reached_prefixes() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        for tail in [
            "joined:{->copy;d.panic(\"stop\")}",
            "joined:{->copy};d.panic(\"stop\");later:{->copy}",
        ] {
            let source = format!(
                "d:@\"debug\";n<{ty}>:{value};p:&n;r:{{->p;->tag:true}};out:r.{{copy:$;{tail}}}"
            );
            let (checker, reports) = checked(&source);
            assert_eq!(reports.slot_uses.len(), 2);
            let op = checker
                .emissions
                .values()
                .find(|op| op.composed.is_some())
                .unwrap();
            let primary = op.composed.unwrap().shared_primary.unwrap();
            let slot = reports.slot_uses[&Port::Emission(op.targets[0].id)].1;
            let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(layout[0].shape, Shape::SharedScalar(primary));
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
        }
    }
}
