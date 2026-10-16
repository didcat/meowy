use super::{super::tests::context, *};

#[test]
pub(crate) fn call_forest_access_precharges_exact_iterator_work_without_payload() {
    let (mut checker, reports) = context("f:(a<int32>,b<int32>){};f(1,2)");
    let site = *checker.invocations.first_key_value().unwrap().0;
    let (forest, _) = checker
        .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    let before = format!("{reports:?}{:?}{:?}", forest.index, checker.edge_counts());
    let start = checker.flow.work;
    let _ = checker
        .call_argument_forest(&forest, site, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    let start = checker.flow.work;
    let (_, inputs) = checker
        .call_argument_forest(&forest, site, Span::default())
        .unwrap();
    let expected = inputs
        .map(|(param, point, row, marker)| (param, point, row.copied(), marker))
        .collect::<Vec<_>>();
    assert_eq!(checker.flow.work - start, work);
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        match checker.call_argument_forest(&forest, site, Span::default()) {
            Ok((_, inputs)) => {
                assert_eq!(short, 0);
                let actual = inputs
                    .map(|(param, point, row, marker)| (param, point, row.copied(), marker))
                    .collect::<Vec<_>>();
                assert_eq!(actual, expected);
            }
            Err(error) => {
                assert_eq!(short, 1);
                assert!(error.message.contains("budget"));
            }
        }
        if short == 0 {
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
        assert_eq!(
            format!("{reports:?}{:?}{:?}", forest.index, checker.edge_counts()),
            before
        );
    }
}

#[test]
pub(crate) fn call_forest_qualification_reuses_one_index_and_bounds_aggregate_work_and_payload() {
    let (mut checker, reports) =
        context("f:(a<int32>,b<int32>){};r:{->n:1};copy:r.n;f(copy,copy);f(copy,copy)");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (_, parts) = checker
        .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    let start = checker.flow.work;
    let left = checker
        .qualify_call_forests(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(left, parts);
    let used = MAX_EDGES - left;
    assert_eq!(
        checker
            .qualify_call_forests(&reports, Span::default(), used)
            .unwrap(),
        0
    );
    assert!(
        checker
            .qualify_call_forests(&reports, Span::default(), used - 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.qualify_call_forests(&reports, Span::default(), used);
        if short == 0 {
            assert_eq!(result.unwrap(), 0);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn call_forest_access_rejects_oversized_arguments_before_inspection() {
    let mut costs = Vec::new();
    for count in [MAX_ITEMS + 1, MAX_ITEMS * 2] {
        let (mut checker, reports) = context("f:(a<int32>){};f(1)");
        let site = *checker.invocations.first_key_value().unwrap().0;
        let (forest, _) = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        let call = checker.invocations.get_mut(&site).unwrap();
        call.args = vec![call.args[0]; count];
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let work = checker.flow.work;
        let Err(error) = checker.call_argument_forest(&forest, site, Span::default()) else {
            panic!()
        };
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("call-forest access budget"));
        costs.push(checker.flow.work - work);
        assert_eq!(checker.invocations[&site].args.len(), count);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    assert_eq!(costs[0], costs[1]);
}
