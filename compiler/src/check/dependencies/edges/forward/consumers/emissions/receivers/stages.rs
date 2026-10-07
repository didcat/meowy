use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn receiver_emission_sources_keep_publication_initialization_and_target_visits_independent()
 {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
    let targets = op.targets.clone();
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 3);
    let slot = expected[&Port::Emission(targets[0].id)].1;
    let source = reports.results[&slot.block].1.dispatch.unwrap();
    let receiver = *checker
        .dispatch_ops
        .iter()
        .find(|(_, op)| matches!(op.receiver, Shape::Record { .. }))
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
pub(crate) fn receiver_emission_sources_keep_initialized_prefixes_before_stopped_destinations() {
    for source in [
        "d:@\"debug\";r:3.{->$;->z:9;->a:true};out:r.{copy:{->$;d.panic(\"stop\")}}",
        "d:@\"debug\";r:{->3;->z:9;->a:true};out:r.{copy:{->$};d.panic(\"stop\")}",
    ] {
        let (mut checker, reports) = checked(source);
        let op = checker
            .emissions
            .values()
            .find(|op| op.composed.is_some())
            .unwrap();
        assert_eq!(reports.slot_uses.len(), 3);
        for target in &op.targets {
            assert!(reports.slot_uses.contains_key(&Port::Emission(target.id)));
        }
        if source.contains("copy:{->$;d.panic") {
            assert!(
                reports
                    .blocks
                    .get(&op.targets[0].block)
                    .is_none_or(|(_, block)| !block.result)
            );
        }
        assert!(
            reports
                .effects
                .values()
                .any(|(_, op)| matches!(op, Effect::Dispatch(op) if op.initialized && !op.result))
        );
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
    }
    let (_, reports) =
        checked("d:@\"debug\";r:{->3;->tag:true};out:r.{d.panic(\"stop\");copy:{->$}}");
    assert!(reports.slot_uses.is_empty());
}

#[test]
pub(crate) fn receiver_emission_sources_keep_direct_values_and_ineligible_producers_opaque() {
    for source in [
        "r:3;out:r.{copy:{->$}}",
        "r:{->3;->tag:true};out:r.{copy:{->named:$}}",
        "r:={->3;->tag:true};out:r.{copy:{->$}}",
        "r:{->3;->tag:=true};out:r.{copy:{->$}}",
        "base:1;r:{->3;->p:&base};out:r.{copy:{->$}}",
        "r:{->3;->tag:true};p:&r;out:(*p).{copy:{->$}}",
        "get<{-><int32>;tag<boolean>}>:(){->3;->tag:true};out:get().{copy:{->$}}",
        "f:(r<{-><int32>;tag<boolean>}>){out:r.{copy:{->$}}}",
        "d:@\"debug\";r<{-><int32>;tag<boolean>}>:3.{->$;->tag:true;d.panic(\"stop\")};out:r.{copy:{->$}}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn receiver_emission_sources_preserve_unknown_empty_multiple_and_partial_layouts() {
    for (kind, source) in [
        (0, "r<{}>:3.{};out:r.{copy:{->$}}"),
        (
            1,
            "flag:=false;r:{|flag|->n:1;|!flag|->n:2};out:r.{copy:{->$}}",
        ),
        (2, "r:{->[1];->n:{->x:2}};out:r.{copy:{->$}}"),
        (3, "r:{->x:1};out:r.{copy<{x<int32>;y<int32>}>:{->$;->y:2}}"),
    ] {
        let (mut checker, reports) = checked(source);
        let op = checker
            .emissions
            .values()
            .find(|op| op.composed.is_some())
            .unwrap();
        assert_eq!(reports.slot_uses.len(), op.targets.len(), "{source}");
        let sources: Vec<_> = op
            .targets
            .iter()
            .map(|target| {
                let slot = reports.slot_uses[&Port::Emission(target.id)].1;
                assert_ne!(slot.block, target.block);
                &reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index]
            })
            .collect();
        match kind {
            0 => {
                assert!(sources.iter().all(
                    |source| matches!(source, Sources::Candidates(values) if values.is_empty())
                ))
            }
            1 => {
                assert!(sources.iter().any(
                    |source| matches!(source, Sources::Candidates(values) if values.len() == 2)
                ))
            }
            2 => assert!(
                sources
                    .iter()
                    .all(|source| matches!(source, Sources::Unknown))
            ),
            3 => {
                let Layout::Slots(slots) = &checker.bodies[&op.targets[0].block].layout else {
                    panic!()
                };
                assert_eq!(slots.len(), 3);
                assert_eq!(op.targets.len(), 2);
            }
            _ => unreachable!(),
        }
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
    }
}
