use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

mod boundaries;
mod evidence;
mod stages;

pub(super) const SOURCE: &str =
    "n:1;p:&n;r:p.{->$;->z:9;->a:true};alias:((r));out:alias.{copy:{->(($))}}";

#[test]
pub(crate) fn shared_local_receiver_emissions_link_exact_slots_without_losing_field_links() {
    for init in ["{->p;->z:9;->a:true}", "p.{->$;->z:9;->a:true}"] {
        for stored in [false, true] {
            let prefix = if stored {
                format!("r:{init};alias:((r));")
            } else {
                String::new()
            };
            let input = if stored { "alias" } else { init };
            let source = format!(
                "n:1;p:&n;flag:=true;{prefix}out:({input}).{{copy:{{->extra:7;->(($)~<{{-><&int32>;a<boolean>;z<int32>}}>)}};|flag|inner:$.{{copy:'outer{{{{'outer->$}}}}}};value:$.a}};f<null>:(p<&int32>){{{prefix}out:({input}).{{copy:{{->$}}}}}}"
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
                    let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                        panic!()
                    };
                    assert_eq!(layout[index].field, target.field);
                    assert_eq!(
                        layout[0].shape,
                        Shape::SharedScalar(op.composed.unwrap().shared_primary.unwrap())
                    );
                    let sources = reports.results[&target.block].1.slots.as_ref().unwrap();
                    assert_eq!(sources[0], Sources::Unknown);
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
                    owners.insert(op.owner);
                    count += 1;
                }
            }
            assert_eq!(count, 9);
            assert_eq!(owners, BTreeSet::from([0, 1]));
            assert_eq!(reports.field_results.len(), 1);
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
}

#[test]
pub(crate) fn shared_local_receiver_emissions_keep_ineligible_storage_and_receiver_copies_opaque() {
    for source in [
        "n:1;p:&n;r:={->p;->tag:true};out:r.{copy:{->$}}",
        "n:1;p:&n;r:{->p;->tag:=true};out:r.{copy:{->$}}",
        "n:1;p:&n;r:{->p;->other:p};out:r.{copy:{->$}}",
        "n:[1];p:&n;r:{->p;->tag:true};out:r.{copy:{->$}}",
        "n:1;p:&n;r:{->p;->tag:true};out:(&r).{copy:{->*$}}",
        "n:1;p:&n;r:{->p;->tag:true};out:r.{alias:$;copy:{->alias}}",
        "n:1;p:&n;r:{->p;->tag:true};out:r.{alias:$;inner:alias.{copy:{->$}}}",
        "n:1;p:&n;r:{->p;->tag:true};out:r.{copy:{->named:$}}",
        "f<null>:(r<{-><&int32>;tag<boolean>}>){out:r.{copy:{->$}}}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
