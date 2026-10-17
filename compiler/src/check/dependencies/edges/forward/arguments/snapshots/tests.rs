use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;

pub(super) fn context(source: &str) -> (Checker, Program, Reports, Facts) {
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    checker.proofs.conditions = checker.guards.clone();
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    (checker, program, reports, facts)
}

#[test]
pub(crate) fn borrow_call_view_keeps_original_points_parameters_and_borrowed_states() {
    let source = "f:(p<&int32>,n<int32>){};g:(p<&int32>){f(p,2)};n:1;f(((&n)),{->3})";
    let (mut checker, program, reports, facts) = context(source);
    let (view, _) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    for &site in view.calls.keys() {
        let (call, inputs) = checker
            .call_borrow_inputs(&view, site, Span::default())
            .unwrap();
        for (position, (param, point, state)) in inputs.enumerate() {
            assert_eq!(point, call.args[position]);
            assert_eq!(
                param,
                reports.signatures.functions[&call.function].params[position]
            );
            let (state, normal) = state.unwrap();
            assert!(normal);
            assert!(std::ptr::eq(
                state,
                &facts.call_inputs[&site].args[position].1
            ));
        }
    }
}

#[test]
pub(crate) fn borrow_call_view_distinguishes_stopped_prefixes_and_missing_snapshots() {
    let source = "d:@\"debug\";f:(a<&int32>,b<int32>,c<int32>){};g<int32>:(){->1};n:1;f(&n,d.panic(\"stop\"),g())";
    let (mut checker, program, mut reports, facts) = context(source);
    reports.effects.clear();
    let (view, _) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(view.calls.len(), 2);
    assert_eq!(facts.call_inputs.len(), 1);
    for &site in view.calls.keys() {
        let (call, inputs) = checker
            .call_borrow_inputs(&view, site, Span::default())
            .unwrap();
        for (position, (_, _, state)) in inputs.enumerate() {
            assert_eq!(state.is_some(), position < 2);
            assert_eq!(state.is_some_and(|(_, normal)| normal), position == 0);
        }
        if call.function == program.functions[1].id {
            assert!(!facts.call_inputs.contains_key(&site));
        }
    }
}

#[test]
pub(crate) fn borrow_call_view_rejects_swapped_sites_reordered_inputs_and_bad_headers() {
    for fault in 0..6 {
        let (mut checker, program, reports, mut facts) =
            context("f:(a<int32>,b<int32>){};f(1,2);f(3,4)");
        let first = *facts.call_inputs.first_key_value().unwrap().0;
        let snapshot = facts.call_inputs.get_mut(&first).unwrap();
        match fault {
            0 => snapshot.header.site += 1,
            1 => snapshot.header.function += 1,
            2 => snapshot.header.body = usize::MAX,
            3 => snapshot.header.count += 1,
            4 => snapshot.args.swap(0, 1),
            5 => snapshot.normal = usize::MAX,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
                .is_err()
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}
