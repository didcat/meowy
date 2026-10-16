use super::*;
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;
use std::collections::BTreeSet;

#[test]
pub(crate) fn call_forest_pairs_keep_parameter_and_caller_identity() {
    let (mut checker, reports) =
        context("f:(a<int32>,unused<int32>){};g:(){r:{->n:1};f(((r.n)),2)};r:{->n:3};f(r.n,4)");
    let calls = checker.invocations.values().cloned().collect::<Vec<_>>();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut owners = BTreeSet::new();
    for call in calls {
        let inputs = checker
            .checked_call_inputs(&reports, call.site, Span::default())
            .unwrap();
        assert_eq!(inputs.call, &call);
        for (position, (param, point)) in inputs.pairs.enumerate() {
            let row = reports.arguments[&point];
            assert_eq!(
                (row.site, row.position, row.param, row.owner),
                (call.site, position, param, call.owner)
            );
            assert_eq!(reports.signatures.params[&param], (call.function, position));
            let marker = reports
                .argument_walk
                .visits
                .iter()
                .find(|visit| matches!(visit, Visit::Argument(_, direct) if direct.point == point))
                .unwrap();
            assert_eq!(
                *marker,
                Visit::Argument(call.owner, reports.argument_sources[&point].1)
            );
            assert!(!reports.initializers.contains_key(&param));
            owners.insert(call.owner);
        }
    }
    assert_eq!(owners.len(), 2);
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn call_forest_pairs_keep_shared_history_outside_the_later_call() {
    let (mut checker, reports) = context("f:(a<int32>){};r:{->n:1};copy:r.n;f(copy);f(copy)");
    let calls = checker.invocations.values().cloned().collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    let mut shared = 0;
    for call in calls {
        let inputs = checker
            .checked_call_inputs(&reports, call.site, Span::default())
            .unwrap();
        let mut pairs = inputs.pairs;
        let (_, point) = pairs.next().unwrap();
        let visits = &reports.argument_walk.visits;
        let start = visits
            .iter()
            .position(|visit| matches!(visit, Visit::Argument(_, direct) if direct.point == point))
            .unwrap();
        if let Visit::Shared(root) = visits[start + 2] {
            assert!(visits[..start].contains(&Visit::Slot(root)));
            assert!(!visits[start..].contains(&Visit::Slot(root)));
            shared += 1;
        }
    }
    assert_eq!(shared, 1);
}

#[test]
pub(crate) fn call_forest_pairs_distinguish_absent_entry_only_unknown_and_known_sources() {
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
        let inputs = checker
            .checked_call_inputs(&reports, call.site, Span::default())
            .unwrap();
        assert_eq!(inputs.pairs.len(), 4);
        for (position, (param, point)) in inputs.pairs.enumerate() {
            assert_eq!(
                param,
                reports.signatures.functions[&call.function].params[position]
            );
            let row = reports.arguments.get(&point);
            let marker =
                reports.argument_walk.visits.iter().find(
                    |visit| matches!(visit, Visit::Argument(_, direct) if direct.point == point),
                );
            assert_eq!(row.is_some(), position < 3 && !(sparse && position == 1));
            assert_eq!(marker.is_some(), position < 2 && !(sparse && position == 1));
            if let Some(Visit::Argument(_, direct)) = marker {
                assert_eq!(direct.source.is_some(), position == 0);
            }
        }
    }
}
