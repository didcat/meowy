use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;
use std::collections::BTreeSet;

#[test]
pub(crate) fn scalar_dispatch_sources_preserve_points_owners_and_distinct_producers() {
    let (mut checker, reports) = checked(
        "n:3.{->$};copy<int32>:((n~<int32>));s:{->copy};r:{->n:1};t:{->r.n};f<int32>:(){n:4.{->$};->((n))}",
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut owners = BTreeSet::new();
    for (&key, &(owner, direct)) in &reports.direct_sources {
        assert!(
            usize::from(direct.source.is_some())
                + usize::from(direct.block.is_some())
                + usize::from(direct.dispatch.is_some())
                <= 1
        );
        let Some(source) = direct.dispatch else {
            continue;
        };
        let input = reports.candidate_inputs[&key].1;
        assert_eq!(source.slot.index, 0);
        assert_eq!(checker.dispatch_ops[&source.point].block, source.slot.block);
        assert_eq!(
            reports.results[&source.slot.block].1.dispatch,
            Some(source.point)
        );
        assert_eq!(direct.point, input.point);
        assert!(!reports.consumers.contains_key(&source.point));
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
                .contains(&Visit::Value(key, input))
        );
        owners.insert(owner);
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert_eq!(
        checker.direct_sources(&reports, Span::default()).unwrap().0,
        reports.direct_sources
    );
    checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn scalar_dispatch_sources_keep_nonscalar_and_unobserved_results_opaque() {
    for source in [
        "v:3.{->x:$};s:{->v}",
        "flag:=false;v:flag.{|$|->1};s:{->v}",
        "n:1;v:(&n).{->$};s:{->v}",
    ] {
        let (_, reports) = checked(source);
        assert!(
            reports
                .direct_sources
                .values()
                .all(|(_, direct)| direct.dispatch.is_none()),
            "{source}"
        );
    }
    let (mut checker, mut reports) = checked("n:3.{->$};s:{->n}");
    let direct = reports
        .direct_sources
        .values()
        .find(|(_, direct)| direct.dispatch.is_some())
        .unwrap()
        .1;
    let source = direct.dispatch.unwrap();
    reports.results.remove(&source.slot.block);
    assert_eq!(
        checker
            .scalar_dispatch_source(&reports, direct.point, 0, Span::default())
            .unwrap(),
        None
    );
}
