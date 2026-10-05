use super::{blocks::seed, seeded::root, *};

#[test]
pub(crate) fn seeded_block_walk_marks_cycles_and_shared_targets_across_source_kinds() {
    for (links, shared) in [
        (vec![vec![Some(0)]], false),
        (vec![vec![Some(1)], vec![Some(2)], vec![Some(0)]], false),
        (
            vec![
                vec![Some(1), Some(2)],
                vec![Some(3)],
                vec![Some(3)],
                vec![None],
            ],
            true,
        ),
    ] {
        let (results, mut inputs, mut sources) = seed(&links);
        let keys: Vec<_> = sources.keys().copied().collect();
        for (index, key) in keys.into_iter().enumerate() {
            let Some(source) = sources[&key].1.block else {
                continue;
            };
            if index % 3 == 1 {
                let direct = &mut sources.get_mut(&key).unwrap().1;
                direct.source = Some(Source {
                    field: source.consumer + 1000,
                    slot: source.slot,
                });
                direct.block = None;
            } else if index % 3 == 2 {
                let input = &mut inputs.get_mut(&key).unwrap().1;
                input.projection = Projection::Primary;
                input.source = Some(source.slot);
                sources.remove(&key);
            }
        }
        let before = format!("{results:?}{inputs:?}{sources:?}");
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        let (walk, _) = graph
            .walk_sources(
                std::iter::once(root(0)),
                Some(&sources),
                &mut Flow::new(),
                Span::default(),
                100,
            )
            .unwrap();
        assert_eq!(
            walk.visits
                .iter()
                .any(|visit| matches!(visit, Visit::Cycle(_))),
            !shared
        );
        assert_eq!(
            walk.visits
                .iter()
                .any(|visit| matches!(visit, Visit::Shared(_))),
            shared
        );
        assert!(
            walk.visits
                .iter()
                .any(|visit| matches!(visit, Visit::Block(_, _, _)))
        );
        if links.len() > 1 {
            assert!(
                walk.visits
                    .iter()
                    .any(|visit| matches!(visit, Visit::Field(_, _, _)))
            );
            assert!(
                walk.visits
                    .iter()
                    .any(|visit| matches!(visit, Visit::Projection(_, _)))
            );
        }
        assert_eq!(format!("{results:?}{inputs:?}{sources:?}"), before);
    }
}

#[test]
pub(crate) fn seeded_block_walk_bounds_peak_storage_depth_and_exact_shared_work() {
    for len in [1, 3, 2048] {
        let links: Vec<_> = (0..len)
            .map(|id| vec![(id + 1 < len).then_some(id + 1)])
            .collect();
        let (results, inputs, sources) = seed(&links);
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        let peak = 4 * len + 1;
        for short in [0, 1] {
            let result = graph.walk_sources(
                std::iter::once(root(0)),
                Some(&sources),
                &mut Flow::new(),
                Span::default(),
                peak - short,
            );
            if short == 0 {
                let (walk, parts) = result.unwrap();
                assert_eq!(walk.visits.len(), 2 * len + 1);
                assert_eq!(parts, len);
            } else {
                assert!(result.unwrap_err().message.contains("budget"));
            }
        }
        let (walk, parts) = graph
            .forest_sources(Some(&sources), &mut Flow::new(), Span::default(), 5 * len)
            .unwrap();
        assert_eq!(parts, 1);
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Slot(_)))
                .count(),
            len
        );
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Shared(_)))
                .count(),
            len - 1
        );
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Block(_, _, _)))
                .count(),
            len - 1
        );
    }
    let (results, inputs, sources) = seed(&[vec![Some(1), Some(1)], vec![None]]);
    let graph = Graph {
        results: &results,
        inputs: &inputs,
    };
    let mut flow = Flow::new();
    let expected = graph
        .forest_sources(Some(&sources), &mut flow, Span::default(), 100)
        .unwrap();
    let work = flow.work;
    let before = format!("{results:?}{inputs:?}{sources:?}");
    for short in [0, 1] {
        flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        flow.full = false;
        let actual = graph.forest_sources(Some(&sources), &mut flow, Span::default(), 100);
        if short == 0 {
            assert_eq!(actual.unwrap(), expected);
        } else {
            assert!(actual.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{results:?}{inputs:?}{sources:?}"), before);
    }
}

#[test]
pub(crate) fn seeded_block_walk_rejects_late_source_and_target_faults_atomically() {
    for fault in 0..8 {
        let (mut results, mut inputs, mut sources) =
            seed(&[vec![Some(1)], vec![Some(2)], vec![None]]);
        let key = (1, 0, 0);
        match fault {
            0 => {
                sources.remove(&key);
            }
            1 => sources.get_mut(&key).unwrap().0 = 1,
            2 => sources.get_mut(&key).unwrap().1.point = usize::MAX,
            3 => {
                sources
                    .get_mut(&key)
                    .unwrap()
                    .1
                    .block
                    .as_mut()
                    .unwrap()
                    .slot
                    .block = usize::MAX
            }
            4 => {
                sources
                    .get_mut(&key)
                    .unwrap()
                    .1
                    .block
                    .as_mut()
                    .unwrap()
                    .slot
                    .index = 1;
                results
                    .get_mut(&2)
                    .unwrap()
                    .1
                    .slots
                    .as_mut()
                    .unwrap()
                    .push(Sources::Unknown);
            }
            5 => results.get_mut(&2).unwrap().0 = 1,
            6 => inputs.get_mut(&key).unwrap().1.source = Some(root(2).slot),
            7 => {
                sources.get_mut(&key).unwrap().1.source = Some(Source {
                    field: 99,
                    slot: root(2).slot,
                })
            }
            _ => unreachable!(),
        }
        let before = format!("{results:?}{inputs:?}{sources:?}");
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        assert!(
            graph
                .walk_sources(
                    std::iter::once(root(0)),
                    Some(&sources),
                    &mut Flow::new(),
                    Span::default(),
                    100
                )
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{results:?}{inputs:?}{sources:?}"), before);
    }
}
