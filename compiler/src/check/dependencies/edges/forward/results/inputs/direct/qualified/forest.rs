use super::*;
use crate::check::dependencies::edges::forward::results::{inputs::graph::Visit, tests::checked};

#[test]
pub(crate) fn expanded_forest_retains_original_reports_opaque_inputs_and_independent_owners() {
    let source = "r:{->n:1};a:{->n:r.n};b:{->a};c:{->(r.n)};d:{->r.n~<int32>};e<int32>:{->r.n};m:={->n:2};u:{->m.n};p:&r;v:{->p.n};f<null>:(){r:{->n:3};s:{->r.n}}";
    let (mut checker, reports) = checked(source);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let original = &reports.candidate_walk.visits;
    let expanded = &reports.expanded_walk.visits;
    let roots = |visits: &[Visit]| {
        visits
            .iter()
            .filter_map(|visit| match visit {
                Visit::Root(root) => Some(*root),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(roots(original), roots(expanded));
    let mut owners = BTreeSet::new();
    for (&key, &(owner, direct)) in &reports.direct_sources {
        let input = reports.candidate_inputs[&key].1;
        assert!(original.contains(&Visit::Value(key, input)));
        if let Some(source) = direct.source {
            assert!(expanded.contains(&Visit::Field(key, input, source)));
            assert!(!expanded.contains(&Visit::Value(key, input)));
            owners.insert(owner);
        } else {
            assert!(expanded.contains(&Visit::Value(key, input)));
        }
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert_eq!(
        expanded
            .iter()
            .filter(|visit| matches!(visit, Visit::Field(_, _, _)))
            .count(),
        5
    );
    assert_eq!(
        expanded
            .iter()
            .filter(|visit| matches!(visit, Visit::Slot(_)))
            .count(),
        roots(expanded).len()
    );
    assert!(
        expanded
            .iter()
            .any(|visit| matches!(visit, Visit::Projection(_, _)))
    );
    assert!(
        !expanded
            .iter()
            .any(|visit| matches!(visit, Visit::Cycle(_)))
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
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn expanded_forest_shares_qualification_traversal_payload_and_work_atomically() {
    let (mut checker, mut reports) = checked("r:{->n:1};a:{->((r.n))};b:{->(r.n)}");
    let (_, left) = checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let scratch = MAX_EDGES - left;
    let work = checker.flow.work;
    let expected = checker
        .direct_walk_limited(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - work;
    assert_eq!(expected.0, reports.expanded_walk);
    assert!(MAX_EDGES - expected.1 > scratch + expected.0.visits.len());
    assert!(
        checker
            .direct_walk_limited(&reports, Span::default(), scratch)
            .unwrap_err()
            .message
            .contains("budget")
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let actual = checker.direct_walk_limited(&reports, Span::default(), MAX_EDGES);
        assert_eq!(actual.is_ok(), short == 0);
        if let Ok(actual) = actual {
            assert_eq!(actual, expected);
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    checker.flow.work = 0;
    checker.flow.full = false;
    let (_, direct) = reports
        .direct_sources
        .values_mut()
        .rev()
        .find(|(_, direct)| direct.source.is_some())
        .unwrap();
    direct.source = None;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .direct_walk_report(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
