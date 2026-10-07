use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;
use std::collections::BTreeSet;

pub(super) const SOURCE: &str = "r:3.{->$;->z:9;->a:true};out:r.{copy:{->(($))}}";

#[test]
pub(crate) fn receiver_emission_sources_keep_slots_names_owners_and_candidate_links() {
    for (init, dispatch) in [
        ("{->3;->z:9;->a:true}", false),
        ("3.{->$;->z:9;->a:true}", true),
    ] {
        let source = format!(
            "r:{init};out:r.{{alias:(($));copy:{{->alias}};inner:$.{{copy:{{->$}}}}}};f<null>:(){{r:{init};out:r.{{copy:{{->$}}}}}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 9);
        let mut owners = BTreeSet::new();
        let mut count = 0;
        for (&id, op) in &checker.emissions {
            if op.composed.is_none() {
                continue;
            }
            let (_, Effect::Emission(observed)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(
                op.targets
                    .iter()
                    .map(|target| target.field.as_deref())
                    .collect::<Vec<_>>(),
                [None, Some("a"), Some("z")]
            );
            for (index, target) in op.targets.iter().enumerate() {
                assert!(observed.initialized[index]);
                let (owner, slot) = reports.slot_uses[&Port::Emission(target.id)];
                assert_eq!((owner, slot.index), (op.owner, index));
                assert_ne!(slot.block, target.block);
                assert_eq!(reports.results[&slot.block].1.dispatch.is_some(), dispatch);
                let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(slots[index].field, target.field);
                let (&key, &(input_owner, input)) = reports
                    .candidate_inputs
                    .iter()
                    .find(|(_, (_, input))| input.candidate.emission == target.id)
                    .unwrap();
                assert_eq!((input_owner, input.source), (owner, Some(slot)));
                for walk in [&reports.candidate_walk, &reports.expanded_walk] {
                    assert!(walk.visits.contains(&Visit::Projection(key, input)));
                    assert!(!walk.visits.contains(&Visit::Unresolved(key, input)));
                }
                owners.insert(owner);
                count += 1;
            }
        }
        assert_eq!(count, 9);
        assert_eq!(owners, BTreeSet::from([0, 1]));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(
            checker
                .candidate_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.candidate_walk
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn receiver_emission_sources_keep_guarded_scopes_and_named_destinations() {
    for source in [
        "flag:=true;r:{->3;->z:9;->a:true};out:r.{|flag|{copy:{->$}};|!flag|$.{copy:{->$}}}",
        "r:{->3;->z:9;->a:true};out:r.{copy:'outer{{'outer->$}};inner:$.{copy:{->$}}}",
    ] {
        let (checker, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), 6);
        for op in checker
            .emissions
            .values()
            .filter(|op| op.composed.is_some())
        {
            for target in &op.targets {
                assert!(reports.slot_uses.contains_key(&Port::Emission(target.id)));
            }
        }
    }
    let (_, reports) = checked(SOURCE);
    assert_eq!(reports.slot_uses.len(), 3);
}
