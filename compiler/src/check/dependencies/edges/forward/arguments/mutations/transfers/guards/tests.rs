use super::*;
use crate::check::dependencies::edges::forward::arguments::mutations::tests::context;

pub(super) fn controlled(source: &str) -> (Checker, Program, Reports, Facts) {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let body = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    let program = Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    checker.proofs.conditions = std::mem::take(&mut checker.guards);
    checker.proofs.tags = std::mem::take(&mut checker.tags);
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    (checker, program, reports, facts)
}

#[test]
pub(crate) fn call_guard_authority_keeps_operation_observation_independent_of_results() {
    let (mut checker, program, mut reports, facts) = context("f:(){};f()");
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
        let (transfers, _) = checker
            .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let expected = if observed.is_some_and(|(operation, _)| operation) {
            Authority::Independent
        } else {
            Authority::Unknown
        };
        assert_eq!(transfers[&point].entry, expected);
        assert_eq!(transfers[&point].observed, observed);
    }
}

#[test]
pub(crate) fn call_guard_authority_keeps_missing_and_stopped_snapshots_unknown() {
    let (mut checker, program, reports, mut facts) = context("f:(p<&!int32>){};n:=1;f(&!n)");
    facts.call_inputs.clear();
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(
        transfers
            .values()
            .all(|transfer| transfer.entry == Authority::Unknown)
    );
    let (mut checker, program, reports, facts) =
        context("d:@\"debug\";f:(p<&!int32>,n<int32>){};n:=1;f(&!n,d.panic(\"stop\"))");
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let transfer = transfers.first_key_value().unwrap().1;
    assert_eq!(transfer.call.snapshot.unwrap().normal, 1);
    assert_eq!(transfer.entry, Authority::Unknown);
    assert!(!transfer.call.inputs[0].1.roots.is_empty());
}

#[test]
pub(crate) fn call_guard_authority_preserves_false_dependent_entries_and_function_boundaries() {
    let source = "flag:false;sink:(p<&!boolean>){};n:=false;|flag|sink(&!n);f:(b<boolean>){v:=false;|b|sink(&!v)}";
    let (mut checker, program, reports, mut facts) = controlled(source);
    let site = checker
        .invocations
        .values()
        .find(|call| call.owner == 0)
        .unwrap()
        .site;
    assert!(checker.invocations[&site].control);
    facts.call_inputs.get_mut(&site).unwrap().header.entered = FALSE;
    checker.proofs.calls.insert(site, FALSE);
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    for transfer in transfers.values() {
        if transfer.call.owner == 0 {
            assert_eq!(transfer.entry, Authority::Dependent);
            assert_eq!(transfer.call.snapshot.unwrap().header.entered, FALSE);
            assert!(!transfer.roots.is_empty());
            assert!(transfer.roots.values().all(|guard| *guard == FALSE));
        } else {
            assert_eq!(transfer.entry, Authority::Independent);
            assert_ne!(transfer.call.snapshot.unwrap().header.entered, FALSE);
        }
    }
    assert_eq!(transfers.len(), 2);
}

#[test]
pub(crate) fn call_guard_authority_inherits_nested_controls_without_tainting_conditions() {
    for op in ["&&", "||"] {
        let source = format!("flag:false;f<boolean>:(){{->true}};flag{op}f();|f()|1");
        let (mut checker, program, reports, facts) = controlled(&source);
        let (transfers, _) = checker
            .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let values = transfers
            .values()
            .map(|transfer| transfer.entry)
            .collect::<Vec<_>>();
        assert_eq!(values, [Authority::Dependent, Authority::Independent]);
    }
    let (mut checker, program, reports, facts) = context("flag:false;f:(){};|flag|f()");
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let (&point, transfer) = transfers.first_key_value().unwrap();
    let branch = *checker.branch_inputs.keys().next().unwrap();
    checker.branch_inputs.remove(&branch);
    assert_eq!(
        checker
            .call_entry_authority(point, transfer, Span::default())
            .unwrap(),
        Authority::Unknown
    );
    checker.invocations.get_mut(&transfer.site).unwrap().control = true;
    assert_eq!(
        checker
            .call_entry_authority(point, transfer, Span::default())
            .unwrap(),
        Authority::Dependent
    );
}
