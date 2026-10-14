use super::*;

#[test]
pub(crate) fn receiver_copy_graphs_distinguish_reference_histories_and_scalar_projection_evidence()
{
    for input in ["$", "copy", "alias"] {
        let source = format!(
            "n:1;p:&n;r:p.{{->$;->z:9;->a:true}};out:r.{{copy:$;alias:((copy));joined:{{->{input}}};value:{{->alias.a}}}}"
        );
        let (mut checker, reports) = checked(&source);
        let op = checker
            .emissions
            .values()
            .find(|op| op.composed.is_some())
            .unwrap();
        let block = op.targets[0].block;
        assert_eq!(
            reports.results[&block].1.slots.as_ref().unwrap()[0],
            Sources::Unknown
        );
        let projections: Vec<_> = reports
            .candidate_inputs
            .iter()
            .filter(|(key, _)| key.0 == block)
            .collect();
        assert_eq!(projections.len(), 2);
        for (&key, &(_, candidate)) in projections {
            assert_ne!(key.1, 0);
            assert!(candidate.source.is_some());
            for walk in [&reports.candidate_walk, &reports.expanded_walk] {
                let visit = Visit::Projection(key, candidate);
                assert!(walk.visits.contains(&visit));
            }
        }
        assert_eq!(reports.field_results.len(), 1);
        assert_eq!(
            checker
                .candidate_inputs(&reports, Span::default())
                .unwrap()
                .0,
            reports.candidate_inputs
        );
        assert_eq!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.expanded_walk
        );
    }
}
