use super::super::{limits::rejected, tests::checked, *};
use crate::check::dependencies::{bodies::completion::Shape, edges::forward::results::Sources};

pub(super) const SOURCE: &str = "early:{->n:1}.n;source:{->7;->a:1;->b:true};copy:{->((source))}";

#[test]
pub(crate) fn emission_slots_keep_sparse_initialization_independent_of_statement_results() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
    let targets = op.targets.clone();
    let prior = reports.slot_uses.clone();
    assert_eq!(prior.len(), 4);
    reports.blocks.remove(&targets[0].block);
    reports.results.remove(&targets[0].block);
    for result in [false, true] {
        for selected in [None, Some(0), Some(2)] {
            let Effect::Emission(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
                panic!()
            };
            op.initialized.fill(false);
            op.result = result;
            if let Some(part) = selected {
                op.initialized[part] = true;
            }
            let expected = prior
                .iter()
                .filter(|(port, _)| match port {
                    Port::Emission(emit) => selected.is_some_and(|part| targets[part].id == *emit),
                    _ => true,
                })
                .map(|(&port, &value)| (port, value))
                .collect::<Uses>();
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                expected
            );
        }
    }
    assert_eq!(reports.slot_uses, prior);
}

#[test]
pub(crate) fn emission_slots_reject_late_targets_and_unobserved_source_suffixes_atomically() {
    for fault in 0..16 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&id, op) = checker
            .emissions
            .iter()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let last = op.targets[2].id;
        let block = reports.slot_uses[&Port::Emission(last)].1.block;
        let input = reports.results[&block].1.consumer.unwrap();
        assert!(checker.fields.keys().all(|field| *field < id));
        let Effect::Emission(op) = &mut reports.effects.get_mut(&id).unwrap().1 else {
            panic!()
        };
        op.initialized = vec![true, false, false];
        op.result = false;
        match fault {
            0 => op.targets[2].field = Some("other".into()),
            1 => {
                op.targets[2].field = Some("other".into());
                checker.emissions.get_mut(&id).unwrap().targets[2].field = Some("other".into());
            }
            2 => {
                let body = checker.bodies.get_mut(&block).unwrap();
                body.completion.result = Shape::Record { fields: 3 };
                reports.blocks.get_mut(&block).unwrap().1.completion = body.completion;
                let Layout::Slots(slots) = &mut body.layout else {
                    panic!()
                };
                let mut extra = slots[2].clone();
                extra.field = Some("extra".into());
                slots.push(extra);
                reports
                    .results
                    .get_mut(&block)
                    .unwrap()
                    .1
                    .slots
                    .as_mut()
                    .unwrap()
                    .push(Sources::Unknown);
            }
            3 | 4 | 14 | 15 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    3 => slots[2].field = Some("other".into()),
                    4 => slots[2].field = None,
                    14 => slots[0].field = Some("primary".into()),
                    15 => slots[0].mutable = true,
                    _ => unreachable!(),
                }
            }
            5 => checker.bodies.get_mut(&block).unwrap().owner += 1,
            6 => reports.results.get_mut(&block).unwrap().0 += 1,
            7 => reports.consumers.get_mut(&input).unwrap().0 += 1,
            8 => reports.blocks.get_mut(&block).unwrap().1.result = false,
            9 => {
                reports
                    .results
                    .get_mut(&block)
                    .unwrap()
                    .1
                    .slots
                    .as_mut()
                    .unwrap()
                    .pop();
            }
            10 => {
                op.initialized.pop();
            }
            11 => {
                let target = &mut checker.emissions.get_mut(&id).unwrap().targets[2];
                target.alias = Some(0);
                target.storage = Some(0);
                op.targets[2] = target.clone();
            }
            12 => {
                checker.emission_sources.insert(last, (id, 0));
            }
            13 => reports.results.get_mut(&block).unwrap().1.consumer = None,
            _ => unreachable!(),
        }
        reports.parts = 0;
        let before = format!("{:?}{:?}{:?}", checker.emissions, checker.bodies, reports);
        let sources = checker.emission_sources.clone();
        rejected(&mut checker, &reports);
        assert_eq!(
            format!("{:?}{:?}{:?}", checker.emissions, checker.bodies, reports),
            before
        );
        assert_eq!(checker.emission_sources, sources);
    }
}

#[test]
pub(crate) fn emission_slots_keep_unknown_layouts_opaque_and_reject_concrete_scalar_sources() {
    for unknown in [false, true] {
        let (mut checker, mut reports) = checked(SOURCE);
        let op = checker
            .emissions
            .values()
            .find(|op| op.composed.is_some())
            .unwrap();
        let block = reports.slot_uses[&Port::Emission(op.targets[0].id)].1.block;
        let body = checker.bodies.get_mut(&block).unwrap();
        if unknown {
            body.completion.result = Shape::Union { members: 2 };
            body.layout = Layout::Unknown;
            reports.results.get_mut(&block).unwrap().1.slots = None;
        } else {
            let Layout::Slots(slots) = &mut body.layout else {
                panic!()
            };
            body.completion.result = slots[0].shape;
            slots.truncate(1);
            reports
                .results
                .get_mut(&block)
                .unwrap()
                .1
                .slots
                .as_mut()
                .unwrap()
                .truncate(1);
        }
        reports.blocks.get_mut(&block).unwrap().1.completion = body.completion;
        if unknown {
            let uses = checker.slot_uses(&reports, Span::default()).unwrap();
            assert_eq!(uses.len(), 1);
            assert!(uses.keys().all(|port| matches!(port, Port::Operation(_))));
        } else {
            rejected(&mut checker, &reports);
        }
    }
}

#[test]
pub(crate) fn emission_slots_deduplicate_visits_and_share_exact_map_and_work_limits() {
    let (mut checker, mut reports) = checked(SOURCE);
    let prior = reports.slot_uses.clone();
    let effects = reports.effects.clone();
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert_eq!(reports.effects, effects);
    let base = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len();
    let limit = base + prior.len();
    reports.parts = 0;
    let before = format!(
        "{reports:?}{:?}{:?}{:?}",
        checker.emissions,
        checker.bodies,
        checker.edge_counts()
    );
    let start = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        prior
    );
    let work = checker.flow.work - start;
    for room in [limit - 1, base - 1] {
        assert!(
            checker
                .slot_uses_limited(&reports, Span::default(), room)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.slot_uses_limited(&reports, Span::default(), limit);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(actual) = result {
            assert_eq!(actual, prior);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
    }
    assert_eq!(
        format!(
            "{reports:?}{:?}{:?}{:?}",
            checker.emissions,
            checker.bodies,
            checker.edge_counts()
        ),
        before
    );
}

#[test]
pub(crate) fn emission_slots_resolve_group_chains_once_per_composition() {
    let mut costs = [[0; 2]; 2];
    for (row, width) in [1, 8].into_iter().enumerate() {
        for (column, depth) in [0, 16].into_iter().enumerate() {
            let fields = (0..width)
                .map(|part| format!(";->n{part}:1"))
                .collect::<String>();
            let source = format!(
                "v:{{->{}{{->7{fields}}}{}}}",
                "(".repeat(depth),
                ")".repeat(depth)
            );
            let (mut checker, reports) = checked(&source);
            let (&id, op) = checker
                .emissions
                .iter()
                .find(|(_, op)| op.composed.is_some())
                .unwrap();
            let owner = op.owner;
            let mut uses = Uses::new();
            let before = checker.flow.work;
            checker
                .emission_slot_uses(
                    &reports,
                    (id, owner),
                    &reports.effects[&id].1,
                    &mut uses,
                    MAX_EDGES,
                    Span::default(),
                )
                .unwrap();
            costs[row][column] = checker.flow.work - before;
            assert_eq!(uses.len(), width + 1);
        }
    }
    let shallow = costs[0][1] - costs[0][0];
    let wide = costs[1][1] - costs[1][0];
    assert!(shallow > 0);
    assert_eq!(wide, shallow);
}
