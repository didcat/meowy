use super::super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_completion_bodies_qualify_never_endpoints_without_claiming_absence() {
    let (mut checker, reports, ports) = context("'out{|true|{'out.leave()};1}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (obs, _) = checker
        .scalar_completion_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let (&block, _) = checker
        .bodies
        .iter()
        .find(|(_, body)| !body.completion.normal)
        .unwrap();
    assert!(!obs.completion.bodies[&block].normal);
    assert_eq!(obs.completion.bodies[&block].control, Some(false));
    assert_eq!(obs.completion.absent.get(&Port::Leave(block)), Some(&0));
    assert!(
        !obs.completion
            .absent
            .contains_key(&Port::BlockNormal(block))
    );
    assert!(
        !obs.completion
            .absent
            .contains_key(&Port::BlockResult(block))
    );
}

#[test]
pub(crate) fn scalar_completion_bodies_keep_missing_sequences_and_incomplete_owners_unknown() {
    for fault in 0..3 {
        let (mut checker, reports, ports) = context("'out{|true|{'out.leave()};1}");
        let block = *checker
            .bodies
            .iter()
            .find(|(_, body)| !body.completion.normal)
            .unwrap()
            .0;
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let (mut sources, parts) = checker
            .scalar_exit_sources(Span::default(), MAX_EDGES)
            .unwrap();
        match fault {
            0 => {
                checker.sequences.remove(&SequenceSource::Block(block));
            }
            1 => {
                checker
                    .sequences
                    .get_mut(&SequenceSource::Block(block))
                    .unwrap()
                    .items[0] = None
            }
            2 => {
                sources.incomplete.insert(0);
            }
            _ => unreachable!(),
        }
        let definitions = definitions::Definitions::default();
        let (obs, _) = checker
            .scalar_completion_bodies(
                &reports,
                &graph,
                &sources,
                Observations::new(&definitions),
                Span::default(),
                parts,
            )
            .unwrap();
        assert!(!obs.completion.bodies.contains_key(&block));
        assert!(!obs.completion.absent.contains_key(&Port::Leave(block)));
    }
}
