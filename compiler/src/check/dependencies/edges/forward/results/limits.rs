use super::{tests::checked, *};
use crate::check::dependencies::edges::forward::effects::Effect;

#[test]
pub(crate) fn result_sources_share_exact_map_payload_and_work_limits() {
    let (mut checker, reports) = checked("row:{->1;->n:2}");
    let expected = reports.results.clone();
    let effects = reports.effects.clone();
    let blocks = reports.blocks.clone();
    let parts = reports.parts;
    let counts = (checker.body_facts, checker.body_slots, checker.body_names);
    let limit = effects.len() + blocks.len() + expected.len();
    let before = checker.flow.work;
    assert_eq!(
        checker
            .result_sources_limited(&reports, Span::default(), limit, 9)
            .unwrap(),
        (expected.clone(), 0)
    );
    let work = checker.flow.work - before;
    for (room, payload) in [
        (limit, 8),
        (limit - 1, 9),
        (effects.len() + blocks.len() - 1, 9),
    ] {
        assert!(
            checker
                .result_sources_limited(&reports, Span::default(), room, payload)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.result_sources_limited(&reports, Span::default(), limit, 9);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(actual) = result {
            assert_eq!(actual, (expected.clone(), 0));
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
    }
    assert_eq!(reports.effects, effects);
    assert_eq!(reports.blocks, blocks);
    assert_eq!(reports.results, expected);
    assert_eq!(reports.parts, parts);
    assert_eq!(
        (checker.body_facts, checker.body_slots, checker.body_names),
        counts
    );
}

#[test]
pub(crate) fn result_sources_charge_unique_descriptors_and_skip_unknown_layout_payload() {
    let (mut checker, mut reports) = checked("row:{->1;->n:2}");
    let expected = reports.results.clone();
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    for (_, effect) in reports.effects.values_mut() {
        if let Effect::Emission(observed) = effect {
            observed.result = false;
        }
    }
    for (_, block) in reports.blocks.values_mut() {
        block.normal = false;
    }
    let (actual, remaining) = checker
        .result_sources_limited(&reports, Span::default(), MAX_EDGES, 9)
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(remaining, 0);
    let count = actual
        .values()
        .filter_map(|(_, row)| row.slots.as_ref())
        .flatten()
        .map(|slot| match slot {
            Sources::Candidates(values) => values.len(),
            Sources::Unknown => 0,
        })
        .sum::<usize>();
    assert_eq!(count, 2);
    let (mut checker, mut reports) = checked("f<int32><null>:(flag<boolean>){|flag|->1}");
    let id = reports.entries[&1].0;
    reports.blocks.retain(|&block, _| block == id);
    let limit = reports.effects.len() + reports.blocks.len() + 1;
    assert_eq!(
        checker
            .result_sources_limited(&reports, Span::default(), limit, 0)
            .unwrap(),
        (
            BTreeMap::from([(
                id,
                (
                    1,
                    Observed {
                        consumer: None,
                        slots: None
                    }
                )
            )]),
            0
        )
    );
}

#[test]
pub(crate) fn result_sources_discard_late_invalid_blocks_and_candidates_atomically() {
    for fault in 0..5 {
        let (mut checker, mut reports) = checked("a:{->1};b:{->2}");
        let id = *reports.blocks.keys().next_back().unwrap();
        let statement = reports
            .effects
            .iter()
            .rev()
            .find_map(|(&id, (_, effect))| matches!(effect, Effect::Emission(_)).then_some(id))
            .unwrap();
        match fault {
            0 => reports.blocks.get_mut(&id).unwrap().1.span.end += 1,
            1 => reports.blocks.get_mut(&id).unwrap().0 += 1,
            2 | 3 => {
                let Effect::Emission(observed) =
                    &mut reports.effects.get_mut(&statement).unwrap().1
                else {
                    panic!()
                };
                if fault == 2 {
                    observed.targets[0].id = usize::MAX;
                } else {
                    observed.initialized.clear();
                }
            }
            4 => {
                let emission = checker.emissions[&statement].targets[0].id;
                checker.emission_sources.get_mut(&emission).unwrap().1 += 1;
            }
            _ => unreachable!(),
        }
        let effects = reports.effects.clone();
        let blocks = reports.blocks.clone();
        let results = reports.results.clone();
        let parts = reports.parts;
        let counts = (checker.body_facts, checker.body_slots, checker.body_names);
        assert!(
            checker
                .result_sources_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(reports.effects, effects);
        assert_eq!(reports.blocks, blocks);
        assert_eq!(reports.results, results);
        assert_eq!(reports.parts, parts);
        assert_eq!(
            (checker.body_facts, checker.body_slots, checker.body_names),
            counts
        );
    }
}
