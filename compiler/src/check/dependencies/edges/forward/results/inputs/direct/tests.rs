use super::{super::super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;

#[test]
pub(crate) fn direct_sources_preserve_candidate_positions_inputs_and_independent_owners() {
    let source = "r:{->n:1};s:{->r.n};t:{->((r.n))};u:{->r};f<null>:(){r:{->n:2};s:{->r.n}}";
    let (mut checker, reports) = checked(source);
    let expected: Vec<_> = reports
        .candidate_inputs
        .iter()
        .filter(|(_, (_, input))| input.projection == Projection::Value)
        .map(|(&key, _)| key)
        .collect();
    assert_eq!(
        reports.direct_sources.keys().copied().collect::<Vec<_>>(),
        expected
    );
    let mut owners = BTreeSet::new();
    for (&key, &(owner, direct)) in &reports.direct_sources {
        let input = reports.candidate_inputs[&key].1;
        assert_eq!(direct.point, input.point);
        assert_eq!(input.source, None);
        assert!(
            reports
                .candidate_walk
                .visits
                .contains(&Visit::Value(key, input))
        );
        if let Some(source) = direct.source {
            assert_eq!(
                reports.field_results[&Port::Normal(source.field)],
                (owner, source.slot)
            );
            owners.insert(owner);
        }
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert_eq!(
        reports
            .direct_sources
            .values()
            .filter(|(_, direct)| direct.source.is_some())
            .count(),
        3
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert_eq!(
        checker
            .direct_sources_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap()
            .0,
        reports.direct_sources
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn direct_sources_keep_missing_field_links_and_other_wrappers_explicit() {
    let source = "r:{->n:1};a:{->r.n};b:{->(r.n)};c:{->r.n~<int32>};d:{->7}";
    let (mut checker, mut reports) = checked(source);
    assert_eq!(
        reports
            .direct_sources
            .values()
            .filter(|(_, direct)| direct.source.is_some())
            .count(),
        3
    );
    let before = reports.direct_sources.clone();
    reports.field_results.clear();
    let (sources, _) = checker
        .direct_sources_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert_eq!(sources.len(), before.len());
    assert!(sources.values().all(|(_, direct)| direct.source.is_none()));
    for (key, (_, direct)) in sources {
        assert_eq!(direct.point, before[&key].1.point);
    }
}
