use super::*;
use crate::check::dependencies::edges::forward::results::inputs::direct::Direct;

pub(super) fn seed(links: &[Vec<Option<usize>>]) -> (Results, Inputs, Directs) {
    let (results, mut inputs) = seeded::seed(links);
    let sources = inputs
        .iter_mut()
        .map(|(&key, (owner, input))| {
            let source = input.source.take().map(|slot| Source {
                field: input.point + 1000,
                slot,
            });
            input.projection = Projection::Value;
            (
                key,
                (
                    *owner,
                    Direct {
                        point: input.point,
                        source,
                        block: None,
                    },
                ),
            )
        })
        .collect();
    (results, inputs, sources)
}

#[test]
pub(crate) fn seeded_field_walk_distinguishes_cycles_and_shared_sources_across_edge_kinds() {
    for (links, shared) in [
        (vec![vec![Some(0)]], false),
        (vec![vec![Some(1)], vec![Some(0)]], false),
        (
            vec![vec![Some(1), Some(2)], vec![Some(0)], vec![None]],
            false,
        ),
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
        let (results, inputs, sources) = seed(&links);
        for mixed in [false, true] {
            let mut inputs = inputs.clone();
            let mut sources = sources.clone();
            if mixed && links.len() > 1 {
                let direct = sources.remove(&(1, 0, 0)).unwrap().1;
                let input = &mut inputs.get_mut(&(1, 0, 0)).unwrap().1;
                input.projection = Projection::Primary;
                input.source = Some(direct.source.unwrap().slot);
            }
            let graph = Graph {
                results: &results,
                inputs: &inputs,
            };
            let (walk, _) = graph
                .walk_sources(
                    std::iter::once(seeded::root(0)),
                    Some(&sources),
                    &mut Flow::new(),
                    Span::default(),
                    100,
                )
                .unwrap();
            let keys: Vec<_> = walk
                .visits
                .iter()
                .filter_map(|visit| match visit {
                    Visit::Field(key, _, _) | Visit::Projection(key, _) | Visit::Value(key, _) => {
                        Some(*key)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(keys.len(), inputs.len());
            assert_eq!(
                keys.into_iter().collect::<BTreeSet<_>>(),
                inputs.keys().copied().collect()
            );
            assert_eq!(
                walk.visits
                    .iter()
                    .filter(|visit| matches!(visit, Visit::Cycle(_)))
                    .count(),
                usize::from(!shared)
            );
            assert_eq!(
                walk.visits
                    .iter()
                    .filter(|visit| matches!(visit, Visit::Shared(_)))
                    .count(),
                usize::from(shared)
            );
            assert!(walk.visits.contains(&if shared {
                Visit::Shared(seeded::root(3))
            } else {
                Visit::Cycle(seeded::root(0))
            }));
            for (&key, &(_, direct)) in &sources {
                if let Some(source) = direct.source {
                    assert!(
                        walk.visits
                            .contains(&Visit::Field(key, inputs[&key].1, source))
                    );
                }
            }
        }
    }
}
