use super::super::graph::Visit;
use super::{super::super::tests::checked, *};
use crate::check::dependencies::{ScalarKind, bodies::completion::Shape};

pub(super) const SOURCE: &str = "n:1;p:&n;r:p.{->$;->z:9;->a:true};copy:((r));out:{->copy}";

#[test]
pub(crate) fn shared_local_emission_graphs_preserve_chains_owners_and_reference_histories() {
    let source = "n:1;p:&n;r:p.{->$;->a:9;->b:true};copy:((r));out:{->copy};deep:{->out};value:{->deep.a};f<null>:(p<&int32>){r:{->p;->a:7;->b:false};out:{->r}}";
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
            .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
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
pub(crate) fn shared_local_emission_graphs_reject_stale_links_and_initializer_evidence() {
    for fault in 0..14 {
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
        let (&read, op) = checker
            .local_reads
            .iter()
            .find(|(read, _)| checker.points[**read].site == checker.points[id].site)
            .unwrap();
        let local = op.local;
        let binding = reports.shared_initializers[&local].init;
        let source = reports.results[&slot.block].1.dispatch.unwrap();
        match fault {
            0 => reports.slot_uses.get_mut(&port).unwrap().1.index += 1,
            1 => {
                reports.shared_initializers.remove(&local);
            }
            2 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .primary = ScalarKind::Bool
            }
            3 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .owner += 1
            }
            4 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            5 | 6 => {
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                if fault == 5 {
                    op.initialized[input.candidate.target] = false;
                } else {
                    op.targets[input.candidate.target].field = Some("wrong".into());
                }
            }
            7 => reports.slot_uses.get_mut(&port).unwrap().1.block = key.0,
            8 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                slots[0].shape = Shape::SharedScalar(ScalarKind::Bool);
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
            11 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .input = None
            }
            12 => {
                reports.effects.remove(&binding.statement);
            }
            13 => {
                reports.effects.remove(&read);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let errors = [
            checker
                .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap_err(),
            checker
                .candidate_graph(&reports, Span::default())
                .err()
                .unwrap(),
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .err()
                .unwrap(),
        ];
        assert!(
            errors
                .iter()
                .all(|error| error.message.contains("identity")),
            "fault {fault}: {errors:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_emission_graphs_keep_missing_links_unresolved_and_other_sources() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&key, &(_, input)) = reports
        .candidate_inputs
        .iter()
        .find(|(_, (_, input))| input.source.is_some())
        .unwrap();
    reports
        .slot_uses
        .remove(&Port::Emission(input.candidate.emission));
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (inputs, _) = checker
        .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert_eq!(inputs[&key].1.source, None);
    assert_eq!(
        inputs
            .values()
            .filter(|(_, input)| input.source.is_some())
            .count(),
        1
    );
    assert!(checker.candidate_graph(&reports, Span::default()).is_err());
    assert!(
        checker
            .direct_graph(&reports, Span::default(), MAX_EDGES)
            .is_err()
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    reports.candidate_inputs = inputs;
    let unresolved = Visit::Unresolved(key, reports.candidate_inputs[&key].1);
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
        assert!(walk.visits.contains(&unresolved));
    }
}

#[test]
pub(crate) fn shared_local_emission_graphs_share_exact_source_cache_and_work_limits() {
    let (mut checker, reports) = checked(SOURCE);
    let keys: Vec<_> = reports
        .candidate_inputs
        .iter()
        .filter(|(_, (_, input))| input.source.is_some())
        .map(|(&key, _)| key)
        .collect();
    assert_eq!(keys.len(), 2);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let mut ctx = Context::new(&reports, 3);
    for key in &keys {
        assert_eq!(
            checker
                .candidate_input(&mut ctx, *key, Span::default())
                .unwrap(),
            reports.candidate_inputs[key]
        );
    }
    let work = checker.flow.work - start;
    assert_eq!(
        (
            ctx.parts,
            ctx.sources.len(),
            ctx.emissions.len(),
            ctx.blocks.len()
        ),
        (0, 1, 1, 1)
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let mut ctx = Context::new(&reports, 3);
        let result: Result<Vec<_>> = keys
            .iter()
            .map(|key| checker.candidate_input(&mut ctx, *key, Span::default()))
            .collect();
        assert_eq!(result.is_ok(), short == 0);
    }
    checker.flow.work = 0;
    checker.flow.full = false;
    let mut ctx = Context::new(&reports, 2);
    assert!(
        checker
            .candidate_input(&mut ctx, keys[0], Span::default())
            .unwrap_err()
            .message
            .contains("candidate-source budget")
    );
    assert!(ctx.sources.is_empty());
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
