use super::{
    seeded::{root, seed},
    *,
};

#[test]
pub(crate) fn seeded_candidate_walk_bounds_aggregate_peak_storage_and_deep_pending_frames() {
    for len in [1, 3, 2048] {
        let links: Vec<_> = (0..len)
            .map(|id| vec![(id + 1 < len).then_some(id + 1)])
            .collect();
        let (results, inputs) = seed(&links);
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        let peak = 4 * len + 1;
        let (walk, parts) = graph
            .walk_roots(
                std::iter::once(root(0)),
                &mut Flow::new(),
                Span::default(),
                peak,
            )
            .unwrap();
        assert_eq!(walk.visits.len(), 2 * len + 1);
        assert_eq!(parts, len);
        let error = graph
            .walk_roots(
                std::iter::once(root(0)),
                &mut Flow::new(),
                Span::default(),
                peak - 1,
            )
            .unwrap_err();
        assert!(error.message.contains("budget"));
    }
    let (results, inputs) = seed(&[vec![]]);
    let graph = Graph {
        results: &results,
        inputs: &inputs,
    };
    for room in 0..4 {
        assert!(
            graph
                .walk_roots(
                    std::iter::once(root(0)),
                    &mut Flow::new(),
                    Span::default(),
                    room
                )
                .is_err()
        );
    }
    assert_eq!(
        graph
            .walk_roots(
                std::iter::once(root(0)),
                &mut Flow::new(),
                Span::default(),
                4
            )
            .unwrap()
            .1,
        0
    );
}

#[test]
pub(crate) fn seeded_candidate_walk_obeys_exact_work_without_partial_output() {
    let (results, inputs) = seed(&[vec![Some(1), Some(1)], vec![None]]);
    let graph = Graph {
        results: &results,
        inputs: &inputs,
    };
    let mut flow = Flow::new();
    let before = flow.work;
    let expected = graph
        .walk_roots(std::iter::once(root(0)), &mut flow, Span::default(), 100)
        .unwrap();
    let work = flow.work - before;
    let before = format!("{results:?}{inputs:?}");
    for short in [0, 1] {
        flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        flow.full = false;
        let actual = graph.walk_roots(std::iter::once(root(0)), &mut flow, Span::default(), 100);
        assert_eq!(actual.is_ok(), short == 0);
        if let Ok(actual) = actual {
            assert_eq!(actual, expected);
        }
        assert_eq!(format!("{results:?}{inputs:?}"), before);
    }
}

#[test]
pub(crate) fn seeded_candidate_walk_rejects_wrong_owners_missing_inputs_and_invalid_targets() {
    for fault in 0..7 {
        let (mut results, mut inputs) = seed(&[vec![Some(1)], vec![None]]);
        let mut start = root(0);
        match fault {
            0 => start.owner = 1,
            1 => start.slot.index = usize::MAX,
            2 => results.get_mut(&1).unwrap().0 = 1,
            3 => {
                inputs.remove(&(1, 0, 0));
            }
            4 => inputs.get_mut(&(1, 0, 0)).unwrap().1.candidate.emission = usize::MAX,
            5 => {
                inputs
                    .get_mut(&(0, 0, 0))
                    .unwrap()
                    .1
                    .source
                    .as_mut()
                    .unwrap()
                    .block = usize::MAX
            }
            6 => inputs.get_mut(&(0, 0, 0)).unwrap().1.projection = Projection::Value,
            _ => unreachable!(),
        }
        let before = format!("{results:?}{inputs:?}");
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        let error = graph
            .walk_roots(
                std::iter::once(start),
                &mut Flow::new(),
                Span::default(),
                100,
            )
            .unwrap_err();
        assert!(error.message.contains("identity"));
        assert_eq!(format!("{results:?}{inputs:?}"), before);
    }
}
