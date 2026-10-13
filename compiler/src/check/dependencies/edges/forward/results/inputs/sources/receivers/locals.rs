use super::*;
use crate::check::dependencies::ScalarKind;

mod limits;

pub(super) const SOURCE: &str =
    "n:1;p:&n;r:p.{->$;->z:9;->a:true};alias:((r));out:alias.{copy:{->(($))}}";

pub(super) fn local(checker: &Checker, reports: &Reports, receiver: PointId) -> hir::LocalId {
    checker
        .local_reads
        .iter()
        .find(|(id, op)| {
            reports.shared_initializers.contains_key(&op.local)
                && checker.points[**id].site == checker.points[receiver].site
        })
        .unwrap()
        .1
        .local
}

#[test]
pub(crate) fn shared_local_receiver_graphs_preserve_composition_chains_and_unknown_references() {
    let source = "n:1;p:&n;r:p.{->$;->a:9;->b:true};alias:((r));out:alias.{copy:{->(($))};deep:{->copy};value:{->deep.a}};f<null>:(p<&int32>){r:{->p;->a:7;->b:false};out:r.{copy:{->$}}}";
    let (mut checker, reports) = checked(source);
    let mut owners = BTreeSet::new();
    let mut origins = BTreeSet::new();
    let mut count = 0;
    for (&key, &(owner, input)) in &reports.candidate_inputs {
        let Some(slot) = input.source else { continue };
        assert_eq!(
            reports.slot_uses[&Port::Emission(input.candidate.emission)],
            (owner, slot)
        );
        assert_eq!(
            reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
            Sources::Unknown
        );
        assert_eq!(
            reports.results[&key.0].1.slots.as_ref().unwrap()[0],
            Sources::Unknown
        );
        assert!(
            !reports
                .candidate_inputs
                .keys()
                .any(|&(block, slot, _)| block == key.0 && slot == 0)
        );
        for walk in [&reports.candidate_walk, &reports.expanded_walk] {
            assert!(walk.visits.contains(&Visit::Projection(key, input)));
        }
        owners.insert(owner);
        origins.insert(reports.results[&slot.block].1.dispatch.is_some());
        count += 1;
    }
    assert_eq!(count, 6);
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert_eq!(origins, BTreeSet::from([false, true]));
    assert!(
        reports
            .expanded_walk
            .visits
            .iter()
            .any(|visit| matches!(visit, Visit::Field(..)))
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert_eq!(
        checker
            .candidate_inputs(&reports, Span::default())
            .unwrap()
            .0,
        reports.candidate_inputs
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

#[test]
pub(crate) fn shared_local_receiver_graphs_reject_stale_links_and_source_qualification() {
    for fault in 0..11 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&key, &(_, input)) = reports
            .candidate_inputs
            .iter()
            .rev()
            .find(|(_, (_, input))| input.source.is_some())
            .unwrap();
        let slot = input.source.unwrap();
        let port = Port::Emission(input.candidate.emission);
        let id = input.candidate.statement;
        let (&cell, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .find(|(_, row)| row.2.is_some())
            .unwrap();
        let local = local(&checker, &reports, receiver);
        let binding = reports.shared_initializers[&local].init;
        match fault {
            0 => reports.slot_uses.get_mut(&port).unwrap().1.index += 1,
            1 => {
                reports.receivers.remove(&cell);
            }
            2 => {
                reports.shared_initializers.remove(&local);
            }
            3 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .fields += 1
            }
            4 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .owner += 1
            }
            5 => {
                checker
                    .operations
                    .get_mut(&binding.statement)
                    .unwrap()
                    .shared = None
            }
            6 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.initialized = false;
            }
            7 => {
                let source = reports.results[&slot.block].1.dispatch.unwrap();
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            8 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                slots[2].mutable = true;
            }
            9 => {
                checker
                    .emissions
                    .get_mut(&id)
                    .unwrap()
                    .composed
                    .as_mut()
                    .unwrap()
                    .shared_primary = Some(ScalarKind::Bool);
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.composed.as_mut().unwrap().shared_primary = Some(ScalarKind::Bool);
            }
            10 => checker.group_inputs.values_mut().last().unwrap().input = input.point,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let errors = [
            checker
                .candidate_inputs(&reports, Span::default())
                .expect_err("candidate collection"),
            checker
                .candidate_walk_report(&reports, Span::default())
                .expect_err("candidate walk"),
            checker
                .direct_walk_report(&reports, Span::default())
                .expect_err("expanded walk"),
        ];
        assert!(
            errors
                .iter()
                .all(|error| error.message.contains("identity")),
            "fault {fault}, key {key:?}: {errors:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_receiver_graphs_require_rebuilt_links_after_missing_initializer_evidence()
 {
    let (mut checker, mut reports) = checked(SOURCE);
    let keys: Vec<_> = reports
        .candidate_inputs
        .iter()
        .filter(|(_, (_, input))| input.source.is_some())
        .map(|(&key, _)| key)
        .collect();
    assert_eq!(keys.len(), 2);
    let receiver = reports
        .receivers
        .values()
        .find(|row| row.2.is_some())
        .unwrap()
        .1;
    let local = local(&checker, &reports, receiver);
    reports.shared_initializers.remove(&local);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(checker.candidate_inputs(&reports, Span::default()).is_err());
    assert!(
        checker
            .direct_walk_report(&reports, Span::default())
            .is_err()
    );
    let uses = checker.slot_uses(&reports, Span::default()).unwrap();
    assert!(uses.is_empty());
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    reports.slot_uses = uses;
    let (inputs, _) = checker.candidate_inputs(&reports, Span::default()).unwrap();
    for key in &keys {
        assert_eq!(inputs[key].1.source, None);
    }
    assert!(
        checker
            .candidate_walk_report(&reports, Span::default())
            .is_err()
    );
    reports.candidate_inputs = inputs;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for walk in [
        checker
            .candidate_walk_report(&reports, Span::default())
            .unwrap()
            .0,
        checker
            .direct_walk_report(&reports, Span::default())
            .unwrap()
            .0,
    ] {
        for key in &keys {
            assert!(
                walk.visits
                    .contains(&Visit::Unresolved(*key, reports.candidate_inputs[key].1))
            );
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
