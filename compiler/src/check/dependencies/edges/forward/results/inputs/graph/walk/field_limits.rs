use super::{field_cycles::seed, seeded::root, *};

#[test]
pub(crate) fn seeded_field_walk_bounds_peak_storage_and_reuses_forest_state() {
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
                .filter(|visit| matches!(visit, Visit::Field(_, _, _)))
                .count(),
            len - 1
        );
    }
}

#[test]
pub(crate) fn seeded_field_walk_charges_exact_lookup_and_shared_traversal_work() {
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
        assert_eq!(actual.is_ok(), short == 0);
        if let Ok(actual) = actual {
            assert_eq!(actual, expected);
        }
        assert_eq!(format!("{results:?}{inputs:?}{sources:?}"), before);
    }
}

#[test]
pub(crate) fn seeded_field_walk_rejects_late_descriptor_target_and_owner_faults_atomically() {
    for fault in 0..7 {
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
                    .source
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
                    .source
                    .as_mut()
                    .unwrap()
                    .slot
                    .index = usize::MAX
            }
            5 => results.get_mut(&2).unwrap().0 = 1,
            6 => inputs.get_mut(&key).unwrap().1.source = Some(root(2).slot),
            _ => unreachable!(),
        }
        let before = format!("{results:?}{inputs:?}{sources:?}");
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        let error = graph
            .walk_sources(
                std::iter::once(root(0)),
                Some(&sources),
                &mut Flow::new(),
                Span::default(),
                100,
            )
            .unwrap_err();
        assert!(error.message.contains("identity"), "fault {fault}");
        assert_eq!(format!("{results:?}{inputs:?}{sources:?}"), before);
    }
}
