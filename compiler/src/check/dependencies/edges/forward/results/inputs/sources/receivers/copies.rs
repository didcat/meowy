use super::*;

mod limits;

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

#[test]
pub(crate) fn receiver_copy_graphs_preserve_composition_chains_and_unknown_aggregate_histories() {
    let (mut checker, reports) = checked(
        "n:1;p:&n;flag:=true;r:{->p;|flag|->a:1;|!flag|->a:2;->z:[true]};out:r.{copy:$;joined:{->copy;->extra:true};deep:{->joined};value:{->deep.a}}",
    );
    let ops: Vec<_> = checker
        .emissions
        .values()
        .filter(|op| op.composed.is_some())
        .collect();
    assert_eq!(ops.len(), 2);
    let slot = reports.slot_uses[&Port::Emission(ops[0].targets[0].id)].1;
    let histories = reports.results[&slot.block].1.slots.as_ref().unwrap();
    assert_eq!(histories[0], Sources::Unknown);
    assert!(matches!(&histories[1], Sources::Candidates(values) if values.len() == 2));
    assert_eq!(histories[2], Sources::Unknown);
    let joined = ops[0].targets[0].block;
    for target in &ops[1].targets {
        assert_eq!(
            reports.slot_uses[&Port::Emission(target.id)].1.block,
            joined
        );
    }
    for op in ops {
        let block = op.targets[0].block;
        assert_eq!(
            reports.results[&block].1.slots.as_ref().unwrap()[0],
            Sources::Unknown
        );
        assert!(
            !reports
                .candidate_inputs
                .keys()
                .any(|key| key.0 == block && key.1 == 0)
        );
    }
    assert!(
        reports
            .expanded_walk
            .visits
            .iter()
            .any(|visit| matches!(visit, Visit::Field(..)))
    );
    assert_eq!(
        checker
            .candidate_walk_report(&reports, Span::default())
            .unwrap()
            .0,
        reports.candidate_walk
    );
    assert_eq!(
        checker
            .direct_walk_report(&reports, Span::default())
            .unwrap()
            .0,
        reports.expanded_walk
    );
}
