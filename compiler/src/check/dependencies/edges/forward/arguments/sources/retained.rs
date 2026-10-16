use super::{super::tests::context, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn retained_argument_source_keys_match_normal_rows_across_callers() {
    let (checker, reports) = context(
        "f:(a<int32>,b<int32>,c<int32>,d<int32>){};g:(){r:{->n:2};f(r.n,({->3}),4.{->$},5)};r:{->n:1};f(((r.n)),({->2}),3.{->$},4)",
    );
    assert_eq!(reports.argument_sources.len(), 8);
    assert_eq!(
        reports
            .argument_sources
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        reports
            .arguments
            .iter()
            .filter_map(|(&point, arg)| arg.normal.then_some(point))
            .collect()
    );
    let mut owners = BTreeSet::new();
    for call in checker.invocations.values() {
        let mut kinds = [0; 4];
        for point in &call.args {
            let (owner, direct) = reports.argument_sources[point];
            assert_eq!(owner, call.owner);
            assert_eq!(direct.point, *point);
            assert_eq!(reports.arguments[point].owner, owner);
            owners.insert(owner);
            let kind = match (direct.source, direct.block, direct.dispatch) {
                (Some(_), None, None) => 0,
                (None, Some(_), None) => 1,
                (None, None, Some(_)) => 2,
                (None, None, None) => 3,
                _ => panic!(),
            };
            kinds[kind] += 1;
        }
        assert_eq!(kinds, [1; 4]);
    }
    assert_eq!(owners.len(), 2);
}

#[test]
pub(crate) fn retained_argument_sources_follow_conservative_rows_without_call_effects() {
    for mode in 0..4 {
        let (mut checker, mut reports) = context("f:(a<int32>,b<int32>){};r:{->n:1};f(r.n,2)");
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let point = call.args[1];
        reports.effects.remove(&call.point);
        match mode {
            0 => reports.arguments.get_mut(&point).unwrap().entry = false,
            1 => reports.arguments.get_mut(&point).unwrap().normal = false,
            2 => {
                reports.arguments.remove(&point);
            }
            3 => reports.arguments.clear(),
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let (sources, _) = checker
            .argument_sources(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        assert_eq!(
            sources.keys().copied().collect::<BTreeSet<_>>(),
            reports
                .arguments
                .iter()
                .filter_map(|(&point, arg)| arg.normal.then_some(point))
                .collect()
        );
        assert_eq!(sources.contains_key(&point), mode == 0);
        assert_eq!(sources.contains_key(&call.args[0]), mode != 3);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn retained_argument_source_keys_survive_missing_independent_producer_evidence() {
    let (mut checker, mut reports) = context("f:(a<int32>,b<int32>){};r:{->n:1};f(r.n,2)");
    let point = checker.invocations.first_key_value().unwrap().1.args[0];
    let field = reports.argument_sources[&point].1.source.unwrap().field;
    reports.field_results.remove(&Port::Normal(field));
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (sources, _) = checker
        .argument_sources(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert!(sources.keys().eq(reports.argument_sources.keys()));
    let direct = sources[&point].1;
    assert_eq!(direct.point, point);
    assert_eq!(
        (direct.source, direct.block, direct.dispatch),
        (None, None, None)
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
