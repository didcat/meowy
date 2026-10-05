use super::{super::super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;

#[test]
pub(crate) fn direct_sources_share_exact_map_cache_payload_and_work_limits() {
    let (mut checker, reports) = checked("r:{->n:1};a:{->r.n};b:{->r.n};c:{->r}");
    let expected = &reports.direct_sources;
    let base = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + reports.slot_uses.len()
        + reports.candidate_inputs.len()
        + reports.field_results.len();
    let limit = base + expected.len();
    let roots = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count();
    let fields = expected
        .values()
        .filter_map(|(_, value)| value.source.map(|source| source.field))
        .collect::<BTreeSet<_>>()
        .len();
    let graph = reports.results.len()
        + reports
            .candidate_inputs
            .values()
            .map(|(_, input)| input.candidate.statement)
            .collect::<BTreeSet<_>>()
            .len()
        + reports
            .candidate_inputs
            .values()
            .filter(|(_, input)| input.source.is_some())
            .map(|(_, input)| input.candidate.statement)
            .collect::<BTreeSet<_>>()
            .len();
    let parts = graph + roots + fields;
    let before = format!("{reports:?}");
    let work = checker.flow.work;
    assert_eq!(
        checker
            .direct_sources_limited(&reports, Span::default(), limit, parts)
            .unwrap(),
        (expected.clone(), 0)
    );
    let work = checker.flow.work - work;
    for (limit, parts) in [(base - 1, parts), (limit - 1, parts), (limit, parts - 1)] {
        assert!(
            checker
                .direct_sources_limited(&reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.direct_sources_limited(&reports, Span::default(), limit, parts);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(result) = result {
            assert_eq!(result, (expected.clone(), 0));
        }
        assert_eq!(format!("{reports:?}"), before);
    }
}

#[test]
pub(crate) fn direct_sources_discard_late_candidate_narrowing_and_field_link_faults() {
    for fault in 0..6 {
        let (mut checker, mut reports) = checked("r:{->n:1};a:{->r.n};b:{->r.n}");
        let (&key, &(_, direct)) = reports
            .direct_sources
            .iter()
            .rev()
            .find(|(_, (_, direct))| direct.source.is_some())
            .unwrap();
        let source = direct.source.unwrap();
        match fault {
            0 => reports.candidate_inputs.get_mut(&key).unwrap().1.point = usize::MAX,
            1 => {
                reports
                    .field_results
                    .get_mut(&Port::Normal(source.field))
                    .unwrap()
                    .0 += 1
            }
            2 => {
                reports
                    .field_results
                    .get_mut(&Port::Normal(source.field))
                    .unwrap()
                    .1
                    .index = 0
            }
            3 => {
                let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&direct.point).unwrap()
                else {
                    panic!()
                };
                op.control = !op.control;
            }
            4 => checker.points[source.field].owner += 1,
            5 => checker.fields.get_mut(&source.field).unwrap().count = 0,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .direct_sources_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn direct_sources_preserve_unknown_empty_and_multiple_histories() {
    for (source, count) in [
        ("s:{->{->n:=1}.n}", None),
        ("r<{n<null>}>:{};s:{->r.n}", Some(0)),
        (
            "flag:=false;r:{|flag|->n:1;|!flag|->n:2};s:{->r.n}",
            Some(2),
        ),
    ] {
        let (_, reports) = checked(source);
        let resolved: Vec<_> = reports
            .direct_sources
            .values()
            .filter_map(|(_, direct)| direct.source)
            .collect();
        assert_eq!(resolved.len(), 1, "{source}");
        let slot = resolved[0].slot;
        let history = &reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index];
        if let Some(count) = count {
            let Sources::Candidates(values) = history else {
                panic!()
            };
            assert_eq!(values.len(), count);
        } else {
            assert_eq!(*history, Sources::Unknown);
        }
    }
}
