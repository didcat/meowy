use super::{tests::context, *};
use crate::borrow_value::{Active, MAX_PARTS};
use crate::flow::MAX_PROOF_WORK;

#[test]
pub(crate) fn borrow_call_view_bounds_combined_payload_and_exact_work_atomically() {
    let (mut checker, program, reports, facts) =
        context("f:(p<&int32>,n<int32>){};n:1;f(&n,2);f(&n,3)");
    let before = checker.flow.work;
    let (_, remaining) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - before;
    let parts = MAX_EDGES - remaining;
    let preserved = format!("{reports:?}{:?}", checker.edge_counts());
    for (limit, extra, pass) in [(parts - 1, 0, false), (parts, 1, false), (parts, 0, true)] {
        checker.flow.work = MAX_PROOF_WORK - work + extra;
        checker.flow.full = false;
        let result = checker.call_snapshots(&program, &reports, &facts, Span::default(), limit);
        assert_eq!(result.is_ok(), pass);
        if let Ok((_, remaining)) = result {
            assert_eq!(remaining, 0);
        }
        assert_eq!(preserved, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn borrow_call_view_rejects_foreign_snapshots_and_invalid_guard_payloads() {
    for fault in 0..8 {
        let (mut checker, program, reports, mut facts) = context("f:(p<&int32>){};n:1;f(&n)");
        let site = *facts.call_inputs.first_key_value().unwrap().0;
        let snapshot = facts.call_inputs.get_mut(&site).unwrap();
        let state = &mut snapshot.args[0].1;
        match fault {
            0 => state.present = usize::MAX,
            1 => state.proof = usize::MAX,
            2 => state.origins[0].guard = usize::MAX,
            3 => {
                state.bounds = state.origins.clone();
                state.bounds[0].guard = usize::MAX;
            }
            4 => state.active.push(Active {
                component: vec![],
                member: 0,
                guard: usize::MAX,
            }),
            5 => state.origins = vec![state.origins[0].clone(); MAX_PARTS + 1],
            6 => {
                snapshot.header.entered = usize::MAX;
                checker.proofs.calls.insert(site, usize::MAX);
            }
            7 => {
                let snapshot = facts.call_inputs.remove(&site).unwrap();
                facts.call_inputs.insert(usize::MAX, snapshot);
            }
            _ => unreachable!(),
        }
        let preserved = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
                .is_err()
        );
        assert_eq!(preserved, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn borrow_call_view_revalidates_live_call_identity_before_access() {
    for fault in 0..4 {
        let (mut checker, program, reports, facts) = context("f:(a<int32>,b<int32>){};f(1,2)");
        let (view, _) = checker
            .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let site = *view.calls.first_key_value().unwrap().0;
        let call = checker.invocations.get_mut(&site).unwrap();
        match fault {
            0 => call.args.swap(0, 1),
            1 => call.owner += 1,
            2 => call.function += 1,
            3 => {
                checker.proofs.calls.insert(site, crate::flow::FALSE);
            }
            _ => unreachable!(),
        }
        let preserved = format!("{reports:?}{:?}", checker.edge_counts());
        let Err(error) = checker.call_borrow_inputs(&view, site, Span::default()) else {
            panic!()
        };
        assert_eq!(error.code, "B001");
        assert_eq!(preserved, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn borrow_call_view_keeps_missing_input_evidence_independent_of_graph_rows() {
    let source = "d:@\"debug\";f:(n<int32>){};g<int32>:(n<int32>){->n};f(d.panic(\"stop\"));g(1)";
    let (mut checker, program, mut reports, facts) = context(source);
    reports.effects.clear();
    reports.arguments.clear();
    let (view, _) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    for &site in view.calls.keys() {
        let (call, mut inputs) = checker
            .call_borrow_inputs(&view, site, Span::default())
            .unwrap();
        let (_, _, state) = inputs.next().unwrap();
        if call.function == program.functions[0].id {
            assert!(state.is_some_and(|(_, normal)| !normal));
        } else {
            assert!(state.is_none());
        }
        assert!(inputs.next().is_none());
    }
}
