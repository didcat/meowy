use super::{super::tests::context, *};

#[test]
pub(crate) fn argument_sources_keep_field_block_dispatch_roots_in_the_caller() {
    let (checker, reports) =
        context("f:(a<int32>,b<int32>,c<int32>){};r:{->n:1};f(((r.n~<int32>)),({->2}),3.{->$})");
    let call = checker.invocations.first_key_value().unwrap().1;
    assert_eq!(reports.argument_sources.len(), 3);
    for point in &call.args {
        let (owner, direct) = reports.argument_sources[point];
        assert_eq!(owner, call.owner);
        assert_eq!(direct.point, *point);
        let param = reports.arguments[point].param;
        assert!(!reports.initializers.contains_key(&param));
        assert!(!reports.shared_initializers.contains_key(&param));
    }
    assert!(reports.argument_sources[&call.args[0]].1.source.is_some());
    assert!(reports.argument_sources[&call.args[1]].1.block.is_some());
    assert!(reports.argument_sources[&call.args[2]].1.dispatch.is_some());
}

#[test]
pub(crate) fn argument_sources_keep_unknowns_and_require_their_own_normal_observation() {
    let (mut checker, mut reports) =
        context("f:(a<int32>,b<int32>,c<int32>){};r:{->n:1};p:r.&n;f(r.n,1,*p)");
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    let point = call.args[0];
    assert!(reports.argument_sources[&point].1.source.is_some());
    for point in &call.args[1..] {
        let direct = reports.argument_sources[point].1;
        assert!(direct.source.is_none() && direct.block.is_none() && direct.dispatch.is_none());
    }
    for (entry, normal) in [(false, true), (true, false)] {
        let arg = reports.arguments.get_mut(&point).unwrap();
        arg.entry = entry;
        arg.normal = normal;
        let mut expected = reports.argument_sources.clone();
        if !normal {
            expected.remove(&point);
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker
                .argument_sources(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap()
                .0,
            expected
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    reports.arguments.remove(&point);
    assert!(
        !checker
            .argument_sources(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap()
            .0
            .contains_key(&point)
    );
}

#[test]
pub(crate) fn argument_sources_keep_earlier_links_before_stopped_later_arguments() {
    let (checker, reports) = context(
        "d:@\"debug\";f:(a<int32>,b<int32>,c<int32>){};r:{->n:1};f(r.n,d.panic(\"stop\"),r.n)",
    );
    let call = checker.invocations.first_key_value().unwrap().1;
    assert!(!reports.effects.contains_key(&call.point));
    assert_eq!(reports.argument_sources.len(), 1);
    assert!(reports.argument_sources[&call.args[0]].1.source.is_some());
    assert!(!reports.arguments[&call.args[1]].normal);
    assert!(!reports.argument_sources.contains_key(&call.args[1]));
    assert!(!reports.argument_sources.contains_key(&call.args[2]));
}
