use super::{
    super::{field_cycles::seed, seeded::root},
    *,
};

pub(super) fn arguments(targets: &[Option<usize>]) -> Arguments {
    targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let point = index + 10000;
            (
                point,
                (
                    0,
                    Direct {
                        point,
                        source: target.map(|block| Source {
                            field: point + 100,
                            slot: root(block).slot,
                        }),
                        block: None,
                        dispatch: None,
                    },
                ),
            )
        })
        .collect()
}

#[test]
pub(crate) fn argument_forest_walk_preserves_mixed_cycles_sharing_and_unknown_arguments() {
    for links in [
        vec![vec![Some(0)]],
        vec![vec![Some(1)], vec![Some(0)]],
        vec![
            vec![Some(1), Some(2)],
            vec![Some(3)],
            vec![Some(3)],
            vec![None],
        ],
    ] {
        let (results, mut inputs, mut sources) = seed(&links);
        if links.len() > 1 {
            let direct = sources.remove(&(1, 0, 0)).unwrap().1;
            let input = &mut inputs.get_mut(&(1, 0, 0)).unwrap().1;
            input.projection = Projection::Primary;
            input.source = Some(direct.source.unwrap().slot);
        }
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        let args = arguments(&[Some(0), Some(0), None]);
        let before = format!("{results:?}{inputs:?}{sources:?}{args:?}");
        let (walk, _) = graph
            .argument_forest(&args, &sources, &mut Flow::new(), Span::default(), 100)
            .unwrap();
        let (prior, _) = graph
            .walk_sources(
                [root(0), root(0)].into_iter(),
                Some(&sources),
                &mut Flow::new(),
                Span::default(),
                100,
            )
            .unwrap();
        let kept = walk
            .visits
            .iter()
            .filter(|visit| !matches!(visit, Visit::Argument(..)))
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(kept, prior.visits);
        assert!(walk.visits.contains(&Visit::Shared(root(0))));
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Cycle(_)))
                .count(),
            usize::from(links.len() < 4)
        );
        let markers = walk
            .visits
            .iter()
            .filter_map(|visit| match visit {
                Visit::Argument(owner, direct) => Some((direct.point, (*owner, *direct))),
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(markers, args);
        assert_eq!(format!("{results:?}{inputs:?}{sources:?}{args:?}"), before);
    }
}

#[test]
pub(crate) fn argument_forest_walk_bounds_deep_peak_storage_and_exact_work() {
    for len in [1, 3, 2048] {
        let links = (0..len)
            .map(|id| vec![(id + 1 < len).then_some(id + 1)])
            .collect::<Vec<_>>();
        let (results, inputs, sources) = seed(&links);
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        let args = arguments(&[Some(0)]);
        let peak = 4 * len + 2;
        let mut flow = Flow::new();
        let expected = graph
            .argument_forest(&args, &sources, &mut flow, Span::default(), peak)
            .unwrap();
        let work = flow.work;
        assert_eq!(expected.0.visits.len(), 2 * len + 2);
        assert_eq!(expected.1, len);
        assert!(
            graph
                .argument_forest(&args, &sources, &mut Flow::new(), Span::default(), peak - 1)
                .unwrap_err()
                .message
                .contains("budget")
        );
        for short in [0, 1] {
            let mut flow = Flow::new();
            flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            let result = graph.argument_forest(&args, &sources, &mut flow, Span::default(), peak);
            if short == 0 {
                assert_eq!(result.unwrap(), expected);
                assert_eq!(flow.work, crate::flow::MAX_PROOF_WORK);
            } else {
                assert!(result.unwrap_err().message.contains("budget"));
            }
        }
    }
}

#[test]
pub(crate) fn argument_forest_walk_charges_unknown_rows_and_rejects_oversized_inputs() {
    let (results, inputs, sources) = seed(&[]);
    let graph = Graph {
        results: &results,
        inputs: &inputs,
    };
    let mut args = arguments(&[None, None]);
    assert_eq!(
        graph
            .argument_forest(&args, &sources, &mut Flow::new(), Span::default(), 2)
            .unwrap()
            .1,
        0
    );
    assert!(
        graph
            .argument_forest(&args, &sources, &mut Flow::new(), Span::default(), 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    let value = *args.first_key_value().unwrap().1;
    args = (0..=MAX_EDGES).map(|point| (point, value)).collect();
    let mut flow = Flow::new();
    let before = flow.work;
    assert!(
        graph
            .argument_forest(&args, &sources, &mut flow, Span::default(), MAX_EDGES)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(flow.work, before);
}

#[test]
pub(crate) fn argument_forest_walk_rejects_late_mixed_descriptors_and_foreign_slots() {
    for fault in 0..7 {
        let (results, inputs, sources) = seed(&[vec![None], vec![None]]);
        let graph = Graph {
            results: &results,
            inputs: &inputs,
        };
        let mut args = arguments(&[Some(0), Some(1)]);
        let (owner, direct) = args.last_entry().unwrap().into_mut();
        match fault {
            0 => direct.point += 1,
            1 => *owner += 1,
            2 => {
                direct.block = Some(Block {
                    consumer: direct.point,
                    slot: root(1).slot,
                })
            }
            3 => direct.source.as_mut().unwrap().slot.block = usize::MAX,
            4 => direct.source.as_mut().unwrap().slot.index = 1,
            5 | 6 => {
                direct.source = None;
                let slot = Slot { block: 1, index: 1 };
                if fault == 5 {
                    direct.block = Some(Block {
                        consumer: direct.point,
                        slot,
                    });
                } else {
                    direct.dispatch = Some(Dispatch {
                        point: direct.point,
                        slot,
                    });
                }
            }
            _ => unreachable!(),
        }
        let before = format!("{results:?}{inputs:?}{sources:?}{args:?}");
        let error = graph
            .argument_forest(
                &args,
                &sources,
                &mut Flow::new(),
                Span::default(),
                MAX_EDGES,
            )
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(format!("{results:?}{inputs:?}{sources:?}{args:?}"), before);
    }
}
