use super::*;
use crate::check::dependencies::edges::forward::arguments::mutations::tests::context;

#[test]
pub(crate) fn mutation_transfers_require_operation_observation_independently_of_results() {
    let (mut checker, program, mut reports, facts) =
        context("f:(a<&!int32>,b<&!int32>){};r:{->a:=1;->b:=2};f(r.&!a,r.&!b)");
    let point = checker.invocations.first_key_value().unwrap().1.point;
    let original = reports.effects[&point].clone();
    for observed in [
        None,
        Some((false, true)),
        Some((true, false)),
        Some((true, true)),
    ] {
        reports.effects.remove(&point);
        if let Some((op, end)) = observed {
            let (owner, mut effect) = original.clone();
            let Effect::Call {
                operation, result, ..
            } = &mut effect
            else {
                panic!()
            };
            *operation = op;
            *result = end;
            reports.effects.insert(point, (owner, effect));
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let (transfers, _) = checker
            .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let transfer = &transfers[&point];
        assert_eq!(transfer.observed, observed);
        let operation = observed.is_some_and(|(op, _)| op);
        assert_eq!(transfer.unknown, operation.then_some(FALSE));
        assert_eq!(transfer.roots.len(), usize::from(operation));
        assert_eq!(transfer.call.inputs.len(), 2);
        assert!(std::ptr::eq(
            transfer.call.snapshot.unwrap(),
            &facts.call_inputs[&transfer.site]
        ));
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn mutation_transfers_preserve_stopped_prefixes_and_missing_snapshots() {
    let (mut checker, program, reports, facts) =
        context("d:@\"debug\";f:(a<&!int32>,b<int32>){};n:=1;f(&!n,d.panic(\"stop\"))");
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let transfer = transfers.first_key_value().unwrap().1;
    assert_eq!(transfer.call.snapshot.unwrap().normal, 1);
    assert!(!transfer.call.inputs[0].1.roots.is_empty());
    assert!(transfer.observed.is_none() && transfer.unknown.is_none() && transfer.roots.is_empty());
    let (mut checker, program, reports, mut facts) = context("f:(p<&!int32>){};n:=1;f(&!n)");
    facts.call_inputs.clear();
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let transfer = transfers.first_key_value().unwrap().1;
    assert_eq!(transfer.observed, Some((true, true)));
    assert!(
        transfer.call.snapshot.is_none() && transfer.unknown.is_none() && transfer.roots.is_empty()
    );
}

#[test]
pub(crate) fn mutation_transfers_keep_entry_guards_and_unknown_input_fallback() {
    let source = "f:(a<&!int32>,b<&!int32>){};g:(flag<boolean>,p<&!int32>){n:=1;|flag|f(&!n,p)}";
    let (mut checker, program, reports, facts) = context(source);
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let transfer = transfers.first_key_value().unwrap().1;
    let entered = transfer.call.snapshot.unwrap().header.entered;
    assert_eq!(transfer.unknown, Some(entered));
    assert!(entered != FALSE && entered != crate::flow::TRUE);
    assert_eq!(transfer.roots.len(), 1);
    assert!(
        transfer
            .roots
            .values()
            .all(|guard| checker.flow.implies(*guard, entered))
    );
    assert!(!transfer.call.inputs[0].1.unknown && transfer.call.inputs[1].1.unknown);
}

#[test]
pub(crate) fn mutation_transfers_bound_complete_payload_and_late_work() {
    let (mut checker, program, reports, facts) =
        context("f:(p<&!int32>){};n:=1;f(&!n);r:{->n:=2;f(&!n)}");
    let start = checker.flow.work;
    let (_, left) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let parts = MAX_EDGES - left;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (limit, extra, pass) in [(parts - 1, 0, false), (parts, 1, false), (parts, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        assert_eq!(
            checker
                .call_mutation_transfers(&program, &reports, &facts, Span::default(), limit)
                .is_ok(),
            pass
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn mutation_transfers_keep_unknown_fallback_when_argument_guards_do_not_cover_entry() {
    for proof in [false, true] {
        let (mut checker, program, reports, mut facts) = context("f:(p<&!int32>){};n:=1;f(&!n)");
        let snapshot = facts.call_inputs.first_entry().unwrap().into_mut();
        let entered = snapshot.header.entered;
        assert_eq!(entered, crate::flow::TRUE);
        let state = &mut snapshot.args[0].1;
        if proof {
            state.proof = FALSE;
        } else {
            state.present = FALSE;
        }
        let (transfers, _) = checker
            .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let transfer = transfers.first_key_value().unwrap().1;
        assert!(!transfer.call.inputs[0].1.unknown);
        assert_eq!(transfer.unknown, Some(entered));
        assert_eq!(transfer.roots.len(), 1);
        assert!(transfer.roots.values().all(|guard| *guard == FALSE));
    }
}
