use super::*;

pub(super) fn seed(links: &[Vec<Option<usize>>]) -> (Results, Inputs) {
    let mut results = Results::new();
    let mut inputs = Inputs::new();
    for (block, links) in links.iter().enumerate() {
        let mut values = Vec::new();
        for (position, source) in links.iter().enumerate() {
            let id = inputs.len();
            let candidate = Candidate {
                emission: id,
                statement: id + 10,
                target: 0,
            };
            values.push(candidate);
            inputs.insert(
                (block, 0, position),
                (
                    0,
                    Input {
                        candidate,
                        point: id + 100,
                        projection: if source.is_some() {
                            Projection::Primary
                        } else {
                            Projection::Value
                        },
                        source: source.map(|block| Slot { block, index: 0 }),
                    },
                ),
            );
        }
        results.insert(
            block,
            (
                0,
                Observed {
                    consumer: None,
                    dispatch: None,
                    slots: Some(vec![Sources::Candidates(values)]),
                },
            ),
        );
    }
    (results, inputs)
}

pub(super) fn root(block: usize) -> Root {
    Root {
        owner: 0,
        slot: Slot { block, index: 0 },
    }
}

#[test]
pub(crate) fn seeded_candidate_walk_keeps_diamond_edges_and_shared_slots_distinct_from_cycles() {
    let (results, inputs) = seed(&[
        vec![Some(1), Some(2)],
        vec![Some(3)],
        vec![Some(3)],
        vec![None],
    ]);
    let graph = Graph {
        results: &results,
        inputs: &inputs,
    };
    let (walk, _) = graph
        .walk_roots(
            std::iter::once(root(0)),
            &mut Flow::new(),
            Span::default(),
            100,
        )
        .unwrap();
    let keys: Vec<_> = walk
        .visits
        .iter()
        .filter_map(|visit| match visit {
            Visit::Projection(key, _) | Visit::Value(key, _) => Some(*key),
            _ => None,
        })
        .collect();
    assert_eq!(
        keys,
        [(0, 0, 0), (1, 0, 0), (3, 0, 0), (0, 0, 1), (2, 0, 0)]
    );
    assert!(walk.visits.contains(&Visit::Shared(root(3))));
    assert!(
        !walk
            .visits
            .iter()
            .any(|visit| matches!(visit, Visit::Cycle(_)))
    );
}

#[test]
pub(crate) fn seeded_candidate_walk_marks_self_mutual_and_branch_cycles_without_losing_other_edges()
{
    for links in [
        vec![vec![Some(0)]],
        vec![vec![Some(1)], vec![Some(0)]],
        vec![vec![Some(1), Some(2)], vec![Some(0)], vec![None]],
    ] {
        let (results, inputs) = seed(&links);
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        let (walk, _) = graph
            .walk_roots(
                std::iter::once(root(0)),
                &mut Flow::new(),
                Span::default(),
                100,
            )
            .unwrap();
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Cycle(_)))
                .count(),
            1
        );
        assert!(walk.visits.contains(&Visit::Cycle(root(0))));
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Value(_, _)))
                .count(),
            usize::from(links.len() == 3)
        );
    }
}

#[test]
pub(crate) fn seeded_candidate_forest_expands_each_slot_once_across_roots() {
    let len = 64;
    let links: Vec<_> = (0..len)
        .map(|id| vec![(id + 1 < len).then_some(id + 1)])
        .collect();
    let (results, inputs) = seed(&links);
    let graph = Graph {
        results: &results,
        inputs: &inputs,
    };
    let (walk, parts) = graph
        .forest(&mut Flow::new(), Span::default(), 5 * len)
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
            .filter(|visit| matches!(visit, Visit::Projection(_, _)))
            .count(),
        len - 1
    );
    assert_eq!(
        walk.visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Value(_, _)))
            .count(),
        1
    );
}
