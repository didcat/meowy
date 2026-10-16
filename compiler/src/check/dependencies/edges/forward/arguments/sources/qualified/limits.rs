use super::*;
use crate::check::dependencies::edges::forward::arguments::tests::context;

#[test]
pub(crate) fn argument_source_view_shares_field_cache_and_charges_exact_work_and_payload() {
    let (mut checker, reports) = context("f:(a<int32>,b<int32>){};r:{->n:1};copy:r.n;f(copy,copy)");
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    let mut ctx = Lookup::new(&reports, MAX_EDGES);
    let first = checker
        .direct_source(&mut ctx, call.args[0], call.owner, Span::default())
        .unwrap();
    let left = ctx.parts;
    let second = checker
        .direct_source(&mut ctx, call.args[1], call.owner, Span::default())
        .unwrap();
    assert_eq!(first.source, second.source);
    assert_eq!(ctx.parts, left);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let (sources, parts) = checker
        .argument_source_view(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - start;
    assert!(std::ptr::eq(sources, &reports.argument_sources));
    assert_eq!(parts, left);
    let (_, created) = checker
        .argument_sources(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert_eq!(created + sources.len(), parts);
    let used = MAX_EDGES - parts;
    assert!(used > 0);
    for parts in [used - 1, used] {
        let result = checker.argument_source_view(&reports, Span::default(), parts);
        if parts == used {
            assert_eq!(result.unwrap().1, 0);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.argument_source_view(&reports, Span::default(), used);
        if short == 0 {
            assert_eq!(result.unwrap().1, 0);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_source_view_borrows_empty_unknown_block_and_dispatch_rows_without_payload() {
    for source in [
        "f:(){};f()",
        "f:(a<int32>){};f(1)",
        "f:(a<int32>){};f(({->1}))",
        "f:(a<int32>){};f(1.{->$})",
    ] {
        let (mut checker, reports) = context(source);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let (sources, parts) = checker
            .argument_source_view(&reports, Span::default(), 0)
            .unwrap();
        assert!(std::ptr::eq(sources, &reports.argument_sources));
        assert_eq!(parts, 0);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_source_view_rejects_oversized_maps_before_reconstruction() {
    for rows in [false, true] {
        let (mut checker, mut reports) = context("f:(a<int32>){};f(1)");
        if rows {
            let arg = *reports.arguments.first_key_value().unwrap().1;
            reports.arguments = (0..=MAX_EDGES).map(|point| (point, arg)).collect();
        } else {
            let direct = *reports.argument_sources.first_key_value().unwrap().1;
            reports.argument_sources = (0..=MAX_EDGES).map(|point| (point, direct)).collect();
        }
        let before = (checker.flow.work, reports.parts, checker.edge_counts());
        let error = checker
            .argument_source_view(&reports, Span::default(), MAX_EDGES)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("budget"));
        assert_eq!(
            (checker.flow.work, reports.parts, checker.edge_counts()),
            before
        );
    }
}
