use super::{context, *};
use crate::check::dependencies::ScalarKind;
use crate::flow::MAX_PROOF_WORK;

#[test]
pub(crate) fn declared_access_view_rejects_missing_extra_and_stale_selected_descriptors() {
    for fault in 0..4 {
        let (mut checker, program, reports, facts) =
            context("f:(n<int32>,p<&int32>){};n:1;f(2,&n)");
        let (mut view, _) = checker
            .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let site = *view.calls.first_key_value().unwrap().0;
        let param = program.functions[0].params[1];
        match fault {
            0 => {
                view.access.remove(&param);
            }
            1 => {
                view.access.insert(usize::MAX, Access::Unknown);
            }
            2 => {
                view.access.insert(param, Access::Value);
            }
            3 => {
                view.access.insert(param, Access::Shared(ScalarKind::Bool));
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}{:?}", checker.edge_counts(), view.access);
        let Err(error) = checker.call_mutation_inputs(&view, site, Span::default()) else {
            panic!()
        };
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity"));
        assert_eq!(
            before,
            format!("{reports:?}{:?}{:?}", checker.edge_counts(), view.access)
        );
    }
}

#[test]
pub(crate) fn declared_access_view_revalidates_full_interiors_with_equal_shallow_shapes() {
    let (mut checker, program, reports, facts) =
        context("f:(a<{n<int32>}>,b<{n<&int32>}>){};n:1;f({->n:2},{->n:&n})");
    let (mut view, _) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let function = &program.functions[0];
    let shapes = &reports.signatures.functions[&function.id].inputs;
    assert_eq!(shapes[0], shapes[1]);
    assert_eq!(view.access[&function.params[0]], Access::Value);
    assert_eq!(view.access[&function.params[1]], Access::Unknown);
    view.access.insert(function.params[1], Access::Value);
    let site = *view.calls.first_key_value().unwrap().0;
    let Err(error) = checker.call_mutation_inputs(&view, site, Span::default()) else {
        panic!()
    };
    assert!(error.message.contains("identity"));
}

#[test]
pub(crate) fn declared_access_map_bounds_exact_payload_and_late_work_atomically() {
    let (mut checker, program, reports, _) = context("f:(n<int32>,p<&{n<int32>}>){}");
    let before = checker.flow.work;
    let (_, remaining) = checker
        .declared_access(&program, &reports, Span::default(), 2)
        .unwrap();
    assert_eq!(remaining, 0);
    let work = checker.flow.work - before;
    let preserved = format!("{reports:?}{:?}", checker.edge_counts());
    for (parts, extra, pass) in [(1, 0, false), (2, 1, false), (2, 0, true)] {
        checker.flow.work = MAX_PROOF_WORK - work + extra;
        checker.flow.full = false;
        assert_eq!(
            checker
                .declared_access(&program, &reports, Span::default(), parts)
                .is_ok(),
            pass
        );
        assert_eq!(preserved, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn declared_access_view_prepays_revalidation_even_without_consuming_inputs() {
    let (mut checker, program, reports, facts) = context("f:(n<int32>,p<&int32>){};n:1;f(2,&n)");
    let (view, _) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let site = *view.calls.first_key_value().unwrap().0;
    let before = checker.flow.work;
    let _ = checker
        .call_mutation_inputs(&view, site, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    let preserved = format!("{reports:?}{:?}", checker.edge_counts());
    for extra in [0, 1] {
        checker.flow.work = MAX_PROOF_WORK - work + extra;
        checker.flow.full = false;
        assert_eq!(
            checker
                .call_mutation_inputs(&view, site, Span::default())
                .is_ok(),
            extra == 0
        );
        assert_eq!(preserved, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn declared_access_view_keeps_global_map_checks_for_empty_calls() {
    let (mut checker, program, reports, facts) = context("f:(){};unused:(p<&int32>){};f()");
    let (mut view, _) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(view.access.len(), 1);
    let site = *view.calls.first_key_value().unwrap().0;
    {
        let (_, inputs) = checker
            .call_mutation_inputs(&view, site, Span::default())
            .unwrap();
        assert_eq!(inputs.len(), 0);
    }
    view.access.clear();
    let Err(error) = checker.call_mutation_inputs(&view, site, Span::default()) else {
        panic!()
    };
    assert!(error.message.contains("identity"));
}
