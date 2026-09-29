use super::{super::tests::checked, *};

#[test]
pub(crate) fn call_effect_index_retains_exact_points_across_nested_calls_and_aliases() {
    let source = "g<int32>:(){->2};f<int32>:(a<int32>,b<int32>){->a+b};alias:f;x:g().(alias,g());recur<int32>:(n<int32>){->recur(n)}";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, false);
    let index = checker.call_effect_index(Span::default()).unwrap();
    assert_eq!(index.len(), 4);
    assert!(checker.invocations.values().any(|call| call.owner != 0));
    for (site, call) in &checker.invocations {
        assert_eq!(index[&call.point], *site);
        assert_eq!(reports.effects[&call.point].0, call.owner);
        assert!(
            matches!(&reports.effects[&call.point].1, Effect::Call { site: found, .. } if found == site)
        );
    }
}

#[test]
pub(crate) fn call_effect_index_rejects_duplicate_and_invalid_call_metadata() {
    for fault in 0..8 {
        let (mut checker, reports) = checked("f<int32>:(){->1};a:f();b:f()", false);
        let (&site, call) = checker.invocations.first_key_value().unwrap();
        let point = call.point;
        match fault {
            0 => checker.invocations.get_mut(&site).unwrap().site = usize::MAX,
            1 => checker.calls = site,
            2 => {
                checker.proofs.calls.remove(&site);
            }
            3 => checker.points[point].complete = false,
            4 => checker.points[point].kind = PointKind::Stmt,
            5 => checker.invocations.get_mut(&site).unwrap().owner = 9,
            6 => checker.invocations.get_mut(&site).unwrap().span = Span::default(),
            7 => {
                let last = checker.invocations.last_entry().unwrap().into_mut();
                last.point = point;
                last.span = checker.points[point].span;
            }
            _ => unreachable!(),
        }
        let counts = checker.edge_counts();
        let error = checker.call_effect_index(Span::default()).unwrap_err();
        assert_eq!(error.code, "B001", "fault {fault}");
        assert!(error.message.contains("identity mismatch"), "fault {fault}");
        assert_eq!(checker.edge_counts(), counts);
        assert!(matches!(reports.effects[&point].1, Effect::Call { .. }));
    }
}

#[test]
pub(crate) fn call_effect_index_bounds_capacity_and_work_without_changing_reports() {
    let (mut checker, reports) = checked("f<int32>:(){->1};a:f();b:f()", false);
    let before = checker.flow.work;
    let expected = checker.call_effect_index(Span::default()).unwrap();
    let work = checker.flow.work - before;
    let counts = checker.edge_counts();
    let effects = reports.effects.clone();
    for limit in [0, 1, 2] {
        let result = checker.call_effect_index_limited(Span::default(), limit);
        assert_eq!(result.is_ok(), limit == 2);
        if let Ok(index) = result {
            assert_eq!(index, expected);
        }
    }
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.call_effect_index(Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        assert_eq!(checker.edge_counts(), counts);
        assert_eq!(reports.effects, effects);
    }
}
