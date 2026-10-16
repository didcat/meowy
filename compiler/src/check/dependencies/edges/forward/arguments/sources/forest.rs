use super::{super::tests::context, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;
use std::collections::BTreeSet;

#[test]
pub(crate) fn argument_forest_retains_original_roots_and_descriptors_across_callers() {
    let (mut checker, reports) = context(
        "f:(a<int32>,b<int32>,c<int32>,d<int32>){};g:(){r:{->n:1};f(r.n,({->r.n}),2.{->$},3)};r:{->n:4};f(r.n,({->r.n}),5.{->$},6)",
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let visits = &reports.argument_walk.visits;
    let args = visits
        .iter()
        .filter_map(|visit| match visit {
            Visit::Argument(owner, direct) => Some((direct.point, (*owner, *direct))),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(args, reports.argument_sources);
    assert_eq!(args.len(), 8);
    assert_eq!(
        args.values()
            .map(|(owner, _)| *owner)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    assert_eq!(
        visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Root(_)))
            .count(),
        6
    );
    for pair in visits.windows(2) {
        if let [Visit::Argument(owner, direct), Visit::Root(root)] = pair {
            let slot = direct
                .source
                .map(|source| source.slot)
                .or_else(|| direct.block.map(|source| source.slot))
                .or_else(|| direct.dispatch.map(|source| source.slot))
                .unwrap();
            assert_eq!(root.owner, *owner);
            assert_eq!(root.slot, slot);
            assert_eq!(reports.results[&slot.block].0, *owner);
        }
    }
    assert_eq!(
        checker
            .argument_walk_report(&reports, Span::default(), MAX_EDGES)
            .unwrap()
            .0,
        reports.argument_walk
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn argument_forest_keeps_unknown_arguments_and_earlier_stopped_call_sources() {
    let (checker, reports) = context(
        "d:@\"debug\";f:(a<int32>,b<int32>,c<int32>,e<int32>){};r:{->n:1};f(r.n,2,d.panic(\"stop\"),r.n)",
    );
    let call = checker.invocations.first_key_value().unwrap().1;
    assert!(!reports.effects.contains_key(&call.point));
    let args = reports
        .argument_walk
        .visits
        .iter()
        .filter_map(|visit| match visit {
            Visit::Argument(owner, direct) => Some((*owner, *direct)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], reports.argument_sources[&call.args[0]]);
    assert_eq!(args[1], reports.argument_sources[&call.args[1]]);
    assert_eq!(
        (args[1].1.source, args[1].1.block, args[1].1.dispatch),
        (None, None, None)
    );
    assert!(!reports.argument_sources.contains_key(&call.args[2]));
    assert!(!reports.argument_sources.contains_key(&call.args[3]));
}

#[test]
pub(crate) fn argument_forest_shares_source_slots_without_merging_argument_identities() {
    let (_, reports) = context("f:(a<int32>,b<int32>){};r:{->n:1};copy:r.n;f(copy,copy)");
    let visits = &reports.argument_walk.visits;
    let args = visits
        .iter()
        .filter_map(|visit| match visit {
            Visit::Argument(_, direct) => Some(*direct),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(args.len(), 2);
    assert_ne!(args[0].point, args[1].point);
    assert_eq!(args[0].source, args[1].source);
    assert_eq!(
        visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Shared(_)))
            .count(),
        1
    );
    assert!(!visits.iter().any(|visit| matches!(visit, Visit::Cycle(_))));
}

#[test]
pub(crate) fn argument_forest_keeps_opaque_inputs_without_source_roots() {
    for source in [
        "f:(p<&int32>){};n:1;f(&n)",
        "f:(r<{n<int32>}>){};r:{->n:1};f(r)",
        "f:(a<int32>){};g:(p<int32>){f(p)}",
        "f:(a<int32>){};r:{->n:1};p:r.&n;f(*p)",
    ] {
        let (_, reports) = context(source);
        assert_eq!(reports.argument_walk.visits.len(), 1);
        let Visit::Argument(owner, direct) = reports.argument_walk.visits[0] else {
            panic!();
        };
        assert_eq!(reports.argument_sources[&direct.point], (owner, direct));
        assert_eq!(
            (direct.source, direct.block, direct.dispatch),
            (None, None, None)
        );
    }
}
