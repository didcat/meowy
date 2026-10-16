use super::{super::tests::context, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn call_forest_access_borrows_observations_in_parameter_order_across_callers() {
    let (mut checker, reports) = context(
        "f:(a<int32>,b<int32>,c<int32>,d<int32>){};g:(){r:{->n:1};f(((r.n)),({->2}),3.{->$},4)};r:{->n:5};f(r.n,({->6}),7.{->$},8)",
    );
    let calls = checker.invocations.values().cloned().collect::<Vec<_>>();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (forest, _) = checker
        .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    let mut owners = BTreeSet::new();
    for expected in calls {
        let (call, inputs) = checker
            .call_argument_forest(&forest, expected.site, Span::default())
            .unwrap();
        assert_eq!(call, &expected);
        assert_eq!(inputs.len(), 4);
        for (position, (param, point, row, marker)) in inputs.enumerate() {
            assert_eq!(
                param,
                reports.signatures.functions[&call.function].params[position]
            );
            assert_eq!(point, call.args[position]);
            assert!(std::ptr::eq(row.unwrap(), &reports.arguments[&point]));
            assert_eq!(
                forest.walk.visits[marker.unwrap()],
                Visit::Argument(call.owner, reports.argument_sources[&point].1)
            );
            owners.insert(call.owner);
            assert!(!reports.initializers.contains_key(&param));
        }
    }
    assert_eq!(owners.len(), 2);
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn call_forest_access_keeps_sparse_and_stopped_arguments_distinct() {
    for sparse in [false, true] {
        let (mut checker, mut reports) = context(
            "d:@\"debug\";f:(a<int32>,b<int32>,c<int32>,e<int32>){};r:{->n:1};f(r.n,2,d.panic(\"stop\"),3)",
        );
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        assert!(!reports.effects.contains_key(&call.point));
        if sparse {
            reports.arguments.remove(&call.args[1]);
            reports.argument_sources.remove(&call.args[1]);
            reports.argument_walk = checker
                .argument_walk_report(&reports, Span::default(), MAX_EDGES)
                .unwrap()
                .0;
        }
        let (forest, _) = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        let (_, inputs) = checker
            .call_argument_forest(&forest, call.site, Span::default())
            .unwrap();
        assert_eq!(inputs.len(), 4);
        for (position, (_, point, row, marker)) in inputs.enumerate() {
            assert_eq!(point, call.args[position]);
            assert_eq!(row.is_some(), position < 3 && !(sparse && position == 1));
            assert_eq!(marker.is_some(), position < 2 && !(sparse && position == 1));
            if let Some(marker) = marker {
                let Visit::Argument(_, direct) = forest.walk.visits[marker] else {
                    panic!()
                };
                assert_eq!(direct.source.is_some(), position == 0);
            }
        }
    }
}

#[test]
pub(crate) fn call_forest_access_preserves_shared_context_across_calls() {
    let (mut checker, reports) = context("f:(a<int32>){};r:{->n:1};copy:r.n;f(copy);f(copy)");
    let sites = checker.invocations.keys().copied().collect::<Vec<_>>();
    let (forest, _) = checker
        .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert!(std::ptr::eq(forest.walk, &reports.argument_walk));
    let mut shared = 0;
    for site in sites {
        let (_, mut inputs) = checker
            .call_argument_forest(&forest, site, Span::default())
            .unwrap();
        let (_, _, _, marker) = inputs.next().unwrap();
        let start = marker.unwrap();
        if let Visit::Shared(root) = forest.walk.visits[start + 2] {
            assert!(forest.walk.visits[..start].contains(&Visit::Slot(root)));
            shared += 1;
        }
    }
    assert_eq!(shared, 1);
}

#[test]
pub(crate) fn call_forest_access_handles_empty_parameter_lists() {
    let (mut checker, reports) = context("f:(){};f()");
    let site = *checker.invocations.first_key_value().unwrap().0;
    let (forest, parts) = checker
        .argument_forest_view(&reports, Span::default(), 0, 0)
        .unwrap();
    let (call, mut inputs) = checker
        .call_argument_forest(&forest, site, Span::default())
        .unwrap();
    assert_eq!(call.site, site);
    assert_eq!(inputs.len(), 0);
    assert!(inputs.next().is_none());
    assert!(forest.walk.visits.is_empty());
    assert_eq!(parts, 0);
}
