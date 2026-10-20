use super::super::tests::context;
use super::*;
use crate::flow::FALSE;

#[test]
pub(crate) fn scalar_call_inputs_keep_direct_targets_and_signature_access_separate() {
    for (source, roots, unknown) in [
        ("f:(){};f()", 0, false),
        ("f:(n<int32>){};f(1)", 0, false),
        ("f:(p<&int32>){};n:1;f(&n)", 0, false),
        ("f:(p<&!int32>){};n:=1;f(&!n)", 1, false),
        ("f:(p<&!int32>){};n:=1;f(((&!n)))", 1, false),
        ("f:(p<&!int32>){};n:=1;p:&!n;f(p)", 1, true),
        ("f:(p<&!int32>){};r:{->n:=1};f(r.&!n)", 1, true),
        ("f:(p<&!int32>){};g:(p<&!int32>){f(p)}", 0, true),
    ] {
        let (mut checker, program, reports, facts) = context(source);
        let (mutations, _) = checker
            .scalar_call_mutations(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let (&point, mutation) = mutations.first_key_value().unwrap();
        let call = &checker.invocations[&mutation.site];
        assert_eq!(point, call.point);
        assert_eq!(mutation.owner, call.owner);
        assert_eq!(mutation.roots.len(), roots, "{source}");
        assert_eq!(mutation.unknown, unknown, "{source}");
        assert!(!mutation.control);
        assert_eq!(
            mutation.input,
            call.args
                .last()
                .map_or(Port::Entry(point), |arg| Port::Normal(*arg))
        );
    }
}

#[test]
pub(crate) fn scalar_call_inputs_never_filter_false_roots_or_false_unknown_fallbacks() {
    for source in [
        "f:(p<&!int32>){};n:=1;f(&!n)",
        "f:(p<&!int32>){};g:(p<&!int32>){f(p)}",
    ] {
        let (mut checker, program, reports, mut facts) = context(source);
        let (&site, snapshot) = facts.call_inputs.first_key_value().unwrap();
        assert_eq!(snapshot.normal, snapshot.header.count);
        facts.call_inputs.get_mut(&site).unwrap().header.entered = FALSE;
        checker.proofs.calls.insert(site, FALSE);
        let (mutations, _) = checker
            .scalar_call_mutations(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let mutation = mutations.first_key_value().unwrap().1;
        if source.contains("f(&!n)") {
            assert_eq!(mutation.roots.len(), 1);
            assert!(!mutation.unknown);
        } else {
            assert!(mutation.unknown);
        }
    }
}

#[test]
pub(crate) fn scalar_call_inputs_require_operations_and_keep_missing_snapshots_unknown() {
    let (mut checker, program, mut reports, mut facts) = context("f:(){};f()");
    let point = checker.invocations.first_key_value().unwrap().1.point;
    facts.call_inputs.clear();
    let (mutations, _) = checker
        .scalar_call_mutations(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(mutations[&point].unknown && mutations[&point].control);
    let (_, crate::check::dependencies::edges::forward::effects::Effect::Call { operation, .. }) =
        reports.effects.get_mut(&point).unwrap()
    else {
        panic!()
    };
    *operation = false;
    assert!(
        checker
            .scalar_call_mutations(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap()
            .0
            .is_empty()
    );
}

#[test]
pub(crate) fn scalar_call_inputs_reserve_complete_payload_and_exact_work() {
    let (mut checker, program, reports, facts) =
        context("f:(p<&!int32>){};a:=1;b:=2;f(&!a);f(&!b)");
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_call_mutations(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result =
            checker.scalar_call_mutations(&program, &reports, &facts, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((mutations, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(mutations, expected);
        }
    }
}
