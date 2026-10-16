use super::*;
use crate::check::dependencies::edges::forward::arguments::tests::context;
use std::collections::BTreeSet;

#[test]
pub(crate) fn argument_source_view_borrows_all_kinds_and_distinct_caller_owners() {
    let (mut checker, reports) = context(
        "f:(a<int32>,b<int32>,c<int32>,d<int32>){};g:(){r:{->n:2};f(r.n,({->3}),4.{->$},5)};r:{->n:1};f(((r.n)),({->2}),3.{->$},4)",
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (sources, parts) = checker
        .argument_source_view(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(std::ptr::eq(sources, &reports.argument_sources));
    assert_eq!(sources.len(), 8);
    assert_eq!(
        sources
            .values()
            .map(|(owner, _)| *owner)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    assert!(parts < MAX_EDGES);
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn argument_source_view_rejects_missing_extra_and_changed_descriptors_atomically() {
    for fault in 0..13 {
        let (mut checker, mut reports) =
            context("f:(a<int32>,b<int32>,c<int32>,d<int32>){};r:{->n:1};f(r.n,({->2}),3.{->$},4)");
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let point = call.args[3];
        let field = reports.argument_sources[&call.args[0]].1.source.unwrap();
        match fault {
            0 => {
                reports.argument_sources.remove(&point);
            }
            1 => {
                reports
                    .argument_sources
                    .insert(usize::MAX, reports.argument_sources[&point]);
            }
            2 => reports.argument_sources.get_mut(&point).unwrap().0 += 1,
            3 => reports.argument_sources.get_mut(&point).unwrap().1.point = call.args[0],
            4 => reports.argument_sources.get_mut(&point).unwrap().1.source = Some(field),
            5 => {
                reports
                    .argument_sources
                    .get_mut(&call.args[0])
                    .unwrap()
                    .1
                    .source = None
            }
            6 => {
                reports
                    .argument_sources
                    .get_mut(&call.args[1])
                    .unwrap()
                    .1
                    .block
                    .as_mut()
                    .unwrap()
                    .slot
                    .index = 1
            }
            7 => {
                reports
                    .argument_sources
                    .get_mut(&call.args[2])
                    .unwrap()
                    .1
                    .dispatch
                    .as_mut()
                    .unwrap()
                    .point = point
            }
            8 => {
                reports
                    .argument_sources
                    .get_mut(&call.args[1])
                    .unwrap()
                    .1
                    .source = Some(field)
            }
            9 => reports.arguments.get_mut(&point).unwrap().normal = false,
            10 => {
                reports.arguments.remove(&point);
            }
            11 => reports.arguments.get_mut(&point).unwrap().param = usize::MAX,
            12 => reports
                .entries
                .get_mut(&call.owner)
                .unwrap()
                .1
                .ports
                .retain(|port| *port != Port::Normal(point)),
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .argument_source_view(&reports, Span::default(), MAX_EDGES)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_source_view_accepts_matching_sparse_and_stopped_prefixes() {
    for mode in 0..4 {
        let (mut checker, mut reports) = context(
            "d:@\"debug\";f:(a<int32>,b<int32>,c<int32>,e<int32>){};r:{->n:1};f(r.n,2,d.panic(\"stop\"),r.n)",
        );
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        assert!(!reports.effects.contains_key(&call.point));
        assert_eq!(reports.argument_sources.len(), 2);
        let point = call.args[1];
        match mode {
            0 => reports.arguments.get_mut(&point).unwrap().entry = false,
            1 => reports.arguments.get_mut(&point).unwrap().normal = false,
            2 => {
                reports.arguments.remove(&point);
            }
            3 => {
                reports.arguments.clear();
                reports.argument_sources.clear();
            }
            _ => unreachable!(),
        }
        if matches!(mode, 1 | 2) {
            reports.argument_sources.remove(&point);
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let (sources, _) = checker
            .argument_source_view(&reports, Span::default(), MAX_EDGES)
            .unwrap();
        assert!(std::ptr::eq(sources, &reports.argument_sources));
        assert_eq!(sources.contains_key(&point), mode == 0);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
