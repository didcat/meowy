use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::Sources;
use std::collections::BTreeSet;

pub(super) const SOURCE: &str = "r:3.{->$;->z:9;->a:true};copy:{->((r))}";

#[test]
pub(crate) fn dispatch_emission_sources_keep_exact_projection_slots_and_independent_owners() {
    let source = "f<{-><int32>;a<boolean>;z<int32>}>:(){r:4.{->$;->z:2;->a:true};s:((r));->s};r:3.{->$;->z:5;->a:false};s:((r));out:'outer{{'outer->s}}";
    let (mut checker, reports) = checked(source);
    let mut owners = BTreeSet::new();
    assert_eq!(reports.slot_uses.len(), 6);
    for (&id, op) in &checker.emissions {
        if op.composed.is_none() {
            continue;
        }
        let (_, Effect::Emission(observed)) = &reports.effects[&id] else {
            panic!()
        };
        for (index, target) in op.targets.iter().enumerate() {
            assert!(observed.initialized[index]);
            let (owner, slot) = reports.slot_uses[&Port::Emission(target.id)];
            assert_eq!((owner, slot.index), (op.owner, index));
            assert_ne!(slot.block, target.block);
            let result = &reports.results[&slot.block].1;
            let dispatch = result.dispatch.unwrap();
            assert_eq!(checker.dispatch_ops[&dispatch].owner, owner);
            assert!(result.consumer.is_none() && !reports.consumers.contains_key(&dispatch));
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(slots[index].field, target.field);
            owners.insert(owner);
        }
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn dispatch_emission_sources_keep_source_publication_and_target_visits_independent() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
    let targets = op.targets.clone();
    let dispatch = *checker.dispatch_ops.keys().next().unwrap();
    let prior = reports.slot_uses.clone();
    assert_eq!(prior.len(), 3);
    reports.blocks.remove(&targets[0].block);
    reports.results.remove(&targets[0].block);
    for initialized in [false, true] {
        for completed in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = completed;
            for result in [false, true] {
                for selected in [None, Some(0), Some(2)] {
                    let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.initialized.fill(false);
                    op.result = result;
                    if let Some(part) = selected {
                        op.initialized[part] = true;
                    }
                    let expected = prior
                        .iter()
                        .filter(|(port, _)| {
                            completed
                                && selected
                                    .is_some_and(|part| **port == Port::Emission(targets[part].id))
                        })
                        .map(|(&port, &slot)| (port, slot))
                        .collect::<Uses>();
                    assert_eq!(
                        checker.slot_uses(&reports, Span::default()).unwrap(),
                        expected
                    );
                }
            }
        }
    }
    assert_eq!(reports.slot_uses, prior);
}

#[test]
pub(crate) fn dispatch_emission_sources_preserve_unknown_slots_and_partial_destination_layouts() {
    for source in [
        "copy:{->(3.{->$;->n:=1})}",
        "copy:{->(3.{->[1];->n:{->x:1}})}",
        "r:3.{->x:$};copy<{x<int32>;y<int32>}>:{->r;->y:2}",
        "r<{}>:3.{};copy:{->r}",
    ] {
        let (checker, reports) = checked(source);
        let op = checker
            .emissions
            .values()
            .find(|op| op.composed.is_some())
            .unwrap();
        assert_eq!(reports.slot_uses.len(), op.targets.len(), "{source}");
        for (index, target) in op.targets.iter().enumerate() {
            let (owner, slot) = reports.slot_uses[&Port::Emission(target.id)];
            assert_eq!((owner, slot.index), (op.owner, index));
            assert_ne!(slot.block, target.block);
            let result = &reports.results[&slot.block].1;
            assert!(result.dispatch.is_some() && result.consumer.is_none());
            if (source.contains("n:=") && index == 1) || source.contains("->[1]") {
                assert_eq!(result.slots.as_ref().unwrap()[index], Sources::Unknown);
            }
        }
    }
}

#[test]
pub(crate) fn dispatch_emission_sources_preserve_initialized_links_before_a_stopped_destination() {
    let (checker, reports) =
        checked("d:@\"debug\";r:3.{->$;->tag:true};out:{->r;d.panic(\"stop\")}");
    let op = checker
        .emissions
        .values()
        .find(|op| op.composed.is_some())
        .unwrap();
    assert!(
        reports
            .blocks
            .get(&op.targets[0].block)
            .is_none_or(|(_, block)| !block.result)
    );
    assert_eq!(reports.slot_uses.len(), 2);
    for target in &op.targets {
        assert!(reports.slot_uses.contains_key(&Port::Emission(target.id)));
    }
}

#[test]
pub(crate) fn dispatch_emission_sources_keep_direct_values_and_opaque_producers_separate() {
    for source in [
        "r:3.{->$};out:{->r}",
        "r:3.{->$;->tag:true};out:{->named:r}",
        "r:=3.{->$;->tag:true};out:{->r}",
        "r:3.{->$;->n:=1};out:{->r}",
        "r:3.{->$;->tag:true};p:&r;out:{->*p}",
        "get<{-><int32>;tag<boolean>}>:(){->3;->tag:true};out:{->get()}",
        "f:(r<{-><int32>;tag<boolean>}>){out:{->r}}",
        "d:@\"debug\";r<{-><int32>;tag<boolean>}>:3.{->$;->tag:true;d.panic(\"stop\")};out:{->r}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
