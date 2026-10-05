use super::*;
use crate::check::dependencies::edges::forward::results::tests::checked;

#[test]
pub(crate) fn block_walk_retains_original_values_across_composition_and_field_sources() {
    let (mut checker, reports) = checked(
        "r:{->n:1};n:{->((r.n))};copy<int32>:((n~<int32>));s:{->value:((copy));->other:n};t:{->s}",
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (&key, &(owner, _)) = reports.candidate_inputs.last_key_value().unwrap();
    let root = Root {
        owner,
        slot: Slot {
            block: key.0,
            index: key.1,
        },
    };
    let (graph, _) = checker.candidate_graph(&reports, Span::default()).unwrap();
    let (walk, _) = graph
        .walk_sources(
            std::iter::once(root),
            Some(&reports.direct_sources),
            &mut checker.flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let mut counts = [0; 4];
    for visit in &walk.visits {
        match visit {
            Visit::Projection(_, _) => counts[0] += 1,
            Visit::Block(key, input, source) => {
                counts[1] += 1;
                assert_eq!(reports.direct_sources[key].1.block, Some(*source));
                assert_eq!(input.projection, Projection::Value);
                assert_eq!(input.source, None);
                assert_eq!(reports.candidate_inputs[key], (owner, *input));
                assert!(
                    reports
                        .candidate_walk
                        .visits
                        .contains(&Visit::Value(*key, *input))
                );
                assert_ne!(input.point, source.consumer);
            }
            Visit::Field(_, _, _) => counts[2] += 1,
            Visit::Value(_, _) => counts[3] += 1,
            _ => {}
        }
    }
    assert_eq!(counts, [1, 1, 1, 1]);
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
pub(crate) fn block_walk_preserves_empty_and_multiple_histories_and_shared_sources() {
    for (source, count) in [
        ("n:{};s:{->n;->echo:n}", 0),
        ("flag:=false;n:{|flag|->1;|!flag|->2};s:{->n;->echo:n}", 2),
    ] {
        let (mut checker, reports) = checked(source);
        let sources: Vec<_> = reports
            .direct_sources
            .values()
            .filter_map(|(owner, direct)| direct.block.map(|source| (*owner, source)))
            .collect();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0], sources[1]);
        let (owner, source) = sources[0];
        let Sources::Candidates(values) = &reports.results[&source.slot.block]
            .1
            .slots
            .as_ref()
            .unwrap()[0]
        else {
            panic!()
        };
        assert_eq!(values.len(), count);
        let target = Root {
            owner,
            slot: source.slot,
        };
        assert!(reports.expanded_walk.visits.contains(&Visit::Slot(target)));
        assert!(
            reports
                .expanded_walk
                .visits
                .contains(&Visit::Shared(target))
        );
        if count == 0 {
            assert!(reports.expanded_walk.visits.contains(&Visit::Empty(target)));
        }
        assert_eq!(
            reports
                .expanded_walk
                .visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Block(_, _, _)))
                .count(),
            2
        );
        assert_eq!(
            checker
                .candidate_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.candidate_walk
        );
    }
}

pub(super) fn seed(links: &[Vec<Option<usize>>]) -> (Results, Inputs, Directs) {
    let (results, inputs, mut sources) = field_cycles::seed(links);
    for (_, direct) in sources.values_mut() {
        if let Some(source) = direct.source.take() {
            direct.block = Some(Block {
                consumer: direct.point + 2000,
                slot: source.slot,
            });
        }
    }
    (results, inputs, sources)
}

#[test]
pub(crate) fn seeded_block_walk_retains_unknown_source_histories() {
    let (mut results, inputs, sources) = seed(&[vec![Some(1)], vec![]]);
    results.get_mut(&1).unwrap().1.slots.as_mut().unwrap()[0] = Sources::Unknown;
    let graph = Graph {
        results: &results,
        inputs: &inputs,
    };
    let target = Root {
        owner: 0,
        slot: Slot { block: 1, index: 0 },
    };
    let (walk, _) = graph
        .forest_sources(Some(&sources), &mut Flow::new(), Span::default(), MAX_EDGES)
        .unwrap();
    assert!(walk.visits.contains(&Visit::Unknown(target)));
    assert_eq!(
        walk.visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Block(_, _, _)))
            .count(),
        1
    );
}
