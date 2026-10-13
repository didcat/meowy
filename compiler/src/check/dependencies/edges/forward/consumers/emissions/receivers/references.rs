use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

mod boundaries;

pub(super) const SOURCE: &str = "n:1;p:&n;out:p.{->$;->z:9;->a:true}.{copy:{->(($))}}";

#[test]
pub(crate) fn shared_receiver_emissions_link_exact_source_slots_without_reference_histories() {
    for init in ["{->p;->z:9;->a:true}", "p.{->$;->z:9;->a:true}"] {
        let source = format!(
            "n:1;p:&n;flag:=true;out:({init}).{{copy:{{->(($)~<{{-><&int32>;a<boolean>;z<int32>}}>)}};|flag|inner:$.{{copy:{{->$}}}}}};f<null>:(p<&int32>){{out:({init}).{{copy:'outer{{{{'outer->$}}}}}}}}"
        );
        let (mut checker, reports) = checked(&source);
        let mut owners = BTreeSet::new();
        let mut count = 0;
        for op in checker
            .emissions
            .values()
            .filter(|op| op.composed.is_some())
        {
            for (index, target) in op.targets.iter().enumerate() {
                let (owner, slot) = reports.slot_uses[&Port::Emission(target.id)];
                assert_eq!((owner, slot.index), (op.owner, index));
                assert_ne!(slot.block, target.block);
                let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(slots[index].field, target.field);
                assert_eq!(
                    slots[0].shape,
                    Shape::SharedScalar(op.composed.unwrap().shared_primary.unwrap())
                );
                assert_eq!(
                    reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                    Sources::Unknown
                );
                if index > 0 {
                    let (&key, &(_, input)) = reports
                        .candidate_inputs
                        .iter()
                        .find(|(_, (_, input))| input.candidate.emission == target.id)
                        .unwrap();
                    assert_eq!(input.source, Some(slot));
                    for walk in [&reports.candidate_walk, &reports.expanded_walk] {
                        assert!(walk.visits.contains(&Visit::Projection(key, input)));
                    }
                }
                owners.insert(owner);
                count += 1;
            }
        }
        assert_eq!(count, 9);
        assert_eq!(owners, BTreeSet::from([0, 1]));
        for (&local, entry) in &reports.receivers {
            if entry.2.is_some() {
                assert!(!reports.eligible.contains(&local));
            }
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
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
pub(crate) fn shared_receiver_emissions_keep_source_receiver_and_destination_stages_independent() {
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
    let receiver = *checker
        .dispatch_ops
        .iter()
        .find(|(_, op)| op.shared_primary.is_some())
        .unwrap()
        .0;
    reports.blocks.remove(&targets[0].block);
    reports.results.remove(&targets[0].block);
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
                for result in [false, true] {
                    for selected in [
                        [false; 3],
                        [true, false, false],
                        [false, false, true],
                        [true; 3],
                    ] {
                        let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap()
                        else {
                            panic!()
                        };
                        op.initialized = selected.to_vec();
                        op.result = result;
                        let wanted = targets
                            .iter()
                            .enumerate()
                            .filter_map(|(index, target)| {
                                let port = Port::Emission(target.id);
                                (published && initialized && selected[index])
                                    .then_some((port, expected[&port]))
                            })
                            .collect::<Uses>();
                        assert_eq!(
                            checker.slot_uses(&reports, Span::default()).unwrap(),
                            wanted
                        );
                    }
                }
            }
        }
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn shared_receiver_emissions_preserve_stopped_prefixes_and_opaque_storage() {
    for tail in [
        "copy:{->$;d.panic(\"stop\")}",
        "copy:{->$};d.panic(\"stop\");later:{->$}",
    ] {
        let source = format!("d:@\"debug\";n:1;p:&n;out:p.{{->$;->tag:true}}.{{{tail}}}");
        let (_, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 2);
    }
    for source in [
        "n:1;p:&n;r:{->p;->tag:true};out:r.{copy:{->$}}",
        "n:1;p:&n;out:{->p;->tag:=true}.{copy:{->$}}",
        "n:1;p:&n;out:{->p;->other:p}.{copy:{->$}}",
        "n:1;p:&n;r:{->p;->tag:true};out:(&r).{copy:{->*$}}",
        "n:[1];p:&n;out:{->p;->tag:true}.{copy:{->$}}",
        "n:1;p:&n;out:p.{->$;->tag:true}.{alias:$;copy:{->alias}}",
        "n:1;p:&n;out:p.{->$;->tag:true}.{copy:{->named:$}}",
        "d:@\"debug\";n:1;p:&n;out:p.{->$;->tag:true}.{d.panic(\"stop\");copy:{->$}}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
