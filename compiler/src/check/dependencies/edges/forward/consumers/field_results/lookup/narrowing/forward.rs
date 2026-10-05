use super::*;
use crate::check::dependencies::{
    CoercionKind,
    edges::forward::{consumers::tests::checked, results::inputs::graph::Visit},
};

#[test]
pub(crate) fn forward_field_sources_preserve_typed_candidates_and_independent_owners() {
    let (mut checker, reports) =
        checked("r:{->n:1};s<int32>:{->((r.n))};f<null>:(){r:{->n:2};s:{->n<int32>:((r.n))}}");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut owners = BTreeSet::new();
    let mut ctx = Lookup::new(&reports, MAX_EDGES);
    for (&key, &(owner, direct)) in &reports.direct_sources {
        if !checker.coercions.contains_key(&direct.point) {
            continue;
        }
        let source = direct.source.expect("typed field source");
        let input = reports.candidate_inputs[&key].1;
        let mut current = direct.point;
        let mut counts = [0; 3];
        while current != source.field {
            current = if let Some(op) = checker.coercions.get(&current) {
                counts[0] += 1;
                assert_eq!(op.kind, CoercionKind::Forward);
                assert!(!op.primary);
                op.input
            } else if let Some(group) = checker.group_inputs.get(&current) {
                counts[1] += 1;
                group.input
            } else {
                counts[2] += 1;
                checker.narrowings[&current].input
            };
        }
        assert_eq!(counts, [3, 2, 1]);
        assert_eq!(direct.point, input.point);
        assert_eq!(input.source, None);
        assert_ne!(direct.point, source.field);
        assert_eq!(
            checker
                .field_narrowing_source(
                    &mut ctx,
                    input.point,
                    owner,
                    Span::default(),
                    counts.into_iter().sum(),
                )
                .unwrap(),
            Some(source)
        );
        assert_eq!(
            reports.field_results[&Port::Normal(source.field)],
            (owner, source.slot)
        );
        assert!(
            reports
                .candidate_walk
                .visits
                .contains(&Visit::Value(key, input))
        );
        assert!(
            reports
                .expanded_walk
                .visits
                .contains(&Visit::Field(key, input, source))
        );
        assert!(
            !reports
                .expanded_walk
                .visits
                .contains(&Visit::Value(key, input))
        );
        assert_eq!(
            checker
                .grouped_consumer(&reports, input.point, owner, Span::default())
                .unwrap(),
            None
        );
        owners.insert(owner);
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert_eq!(
        checker.direct_sources(&reports, Span::default()).unwrap().0,
        reports.direct_sources
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
pub(crate) fn forward_field_sources_expand_typed_chains_without_selecting_histories() {
    for (source, count) in [
        ("r:{->n:1};a:{->n<int32>:(r.n)};b<int32>:{->((a.n))}", 2),
        ("r:{->n:=1};a:{->n<int32>:(r.n)};b<int32>:{->((a.n))}", 1),
    ] {
        let (mut checker, reports) = checked(source);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let linked: Vec<_> = reports
            .direct_sources
            .iter()
            .filter(|(_, (_, direct))| direct.source.is_some())
            .collect();
        assert_eq!(linked.len(), count, "{source}");
        for (&key, &(_, direct)) in linked {
            let input = reports.candidate_inputs[&key].1;
            let source = direct.source.unwrap();
            assert!(checker.coercions.contains_key(&input.point));
            assert!(
                reports
                    .candidate_walk
                    .visits
                    .contains(&Visit::Value(key, input))
            );
            assert!(
                reports
                    .expanded_walk
                    .visits
                    .contains(&Visit::Field(key, input, source))
            );
            assert!(
                reports.expanded_walk.visits.iter().any(|visit| {
                    matches!(visit, Visit::Slot(root) if root.slot == source.slot)
                })
            );
        }
        assert_eq!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.expanded_walk
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
