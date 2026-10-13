use super::*;
use crate::check::dependencies::{ScalarKind, bodies::completion::Shape};

pub(super) const SOURCE: &str = "n:1;p:&n;out:p.{->$;->z:9;->a:true}.{copy:{->(($))}}";

#[test]
pub(crate) fn shared_emission_graphs_revalidate_referents_links_scopes_and_source_completion() {
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
        let (&local, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .find(|(_, entry)| entry.2.is_some())
            .unwrap();
        let source = reports.results[&slot.block].1.dispatch.unwrap();
        match fault {
            0 => reports.slot_uses.get_mut(&port).unwrap().1.index += 1,
            1 => {
                reports.receivers.remove(&local);
            }
            2 => {
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
            3 | 4 => {
                let (_, Effect::Dispatch(op)) = reports
                    .effects
                    .get_mut(&(if fault == 3 { source } else { receiver }))
                    .unwrap()
                else {
                    panic!()
                };
                if fault == 3 {
                    op.result = false;
                } else {
                    op.initialized = false;
                }
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
            9 => reports.receivers.get_mut(&local).unwrap().0 += 1,
            10 => {
                let group = checker.group_inputs.values_mut().last().unwrap();
                group.input = input.point;
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let errors = [
            checker
                .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .err()
                .expect("candidate collection"),
            checker
                .candidate_graph(&reports, Span::default())
                .err()
                .expect("candidate graph"),
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .err()
                .expect("expanded graph"),
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
pub(crate) fn shared_emission_graphs_keep_missing_scalar_links_unresolved_and_references_unknown() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&key, &(_, input)) = reports
        .candidate_inputs
        .iter()
        .find(|(_, (_, input))| input.source.is_some())
        .unwrap();
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
    reports
        .slot_uses
        .remove(&Port::Emission(input.candidate.emission));
    let before = format!("{reports:?}");
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
    assert_eq!(format!("{reports:?}"), before);
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
pub(crate) fn shared_emission_graphs_share_exact_source_cache_and_work_limits() {
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
