use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::Sources;
use std::collections::BTreeSet;

mod boundaries;
mod limits;

pub(super) const SOURCE: &str = "n:1;p:&n;r:p.{->$;->z:9;->a:true};copy:((r));out:{->copy}";

#[test]
pub(crate) fn shared_local_emissions_link_exact_typed_slots_through_copies_and_owners() {
    for (ty, value) in [
        ("int8", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        for init in ["{->p;->z:9;->a:true}", "p.{->$;->z:9;->a:true}"] {
            let source = format!(
                "n<{ty}>:{value};p:&n;r:{init};alias:((r));out:'outer{{{{'outer->(alias~<{{-><&{ty}>;a<boolean>;z<int32>}}>)}}}};flag:=true;|flag|other:{{->r}};f<null>:(p<&{ty}>){{r:{init};out:{{->r}}}}"
            );
            crate::compile(&source).unwrap();
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
                    owners.insert(owner);
                    count += 1;
                }
            }
            assert_eq!(count, 9);
            assert_eq!(reports.slot_uses.len(), count);
            assert_eq!(owners, BTreeSet::from([0, 1]));
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
}

#[test]
pub(crate) fn shared_local_emissions_preserve_source_target_and_destination_stages() {
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
    reports.blocks.remove(&targets[0].block);
    reports.results.remove(&targets[0].block);
    for initialized in [false, true] {
        for published in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = published;
            for result in [false, true] {
                for selected in [
                    [false; 3],
                    [true, false, false],
                    [false, false, true],
                    [true; 3],
                ] {
                    let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.initialized = selected.to_vec();
                    op.result = result;
                    let wanted = targets
                        .iter()
                        .enumerate()
                        .filter_map(|(index, target)| {
                            let port = Port::Emission(target.id);
                            (published && selected[index]).then_some((port, expected[&port]))
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
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn shared_local_emissions_keep_stopped_prefixes_and_excluded_storage_opaque() {
    for tail in [
        "out:{->r;d.panic(\"stop\")}",
        "out:{->r};d.panic(\"stop\");later:{->r}",
    ] {
        let source = format!("d:@\"debug\";n:1;p:&n;r:{{->p;->tag:true}};{tail}");
        let (_, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 2);
    }
    for source in [
        "n:1;p:&n;r:={->p;->tag:true};out:{->r}",
        "n:1;p:&n;r:{->p;->tag:=true};out:{->r}",
        "n:1;p:&n;r:{->p;->other:p};out:{->r}",
        "n:[1];p:&n;r:{->p;->tag:true};out:{->r}",
        "n:1;p:&n;r:{->p;->tag:true};out:{->*(&r)}",
        "n:1;p:&n;r:{->p;->tag:true};out:{->named:r}",
        "n:1;p:&n;r:{->p;->tag:true};out:{->named:r;copy:{->named}}",
        "n:1;p:&n;r:{->p;->tag:true};out:r.{alias:=$;copy:{->alias}}",
        "f<{-><&int32>;tag<boolean>}>:(p<&int32>){->p;->tag:true};n:1;r:f(&n);out:{->r}",
        "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true};out:{d.panic(\"stop\");->r}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
