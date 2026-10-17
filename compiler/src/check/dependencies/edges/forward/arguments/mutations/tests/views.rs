use super::{context, *};

#[test]
pub(crate) fn declared_access_view_preserves_shared_contracts_for_exclusive_arguments() {
    let (mut checker, program, reports, facts) = context("f:(p<&int32>){};n:=1;f(&!n)");
    let (view, _) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let site = *view.calls.first_key_value().unwrap().0;
    let (call, mut inputs) = checker
        .call_mutation_inputs(&view, site, Span::default())
        .unwrap();
    let (param, point, access, state) = inputs.next().unwrap();
    assert_eq!(param, program.functions[0].params[0]);
    assert_eq!(point, call.args[0]);
    assert!(matches!(access, Access::Shared(_)));
    let (state, normal) = state.unwrap();
    assert!(normal && std::ptr::eq(state, &facts.call_inputs[&site].args[0].1));
    assert!(inputs.next().is_none());
}

#[test]
pub(crate) fn declared_access_view_does_not_specialize_reference_union_domains() {
    let (mut checker, program, reports, facts) = context("f:(p<&int32><null>){};n:1;f(null);f(&n)");
    let (view, _) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(view.calls.len(), 2);
    for &site in view.calls.keys() {
        let (_, mut inputs) = checker
            .call_mutation_inputs(&view, site, Span::default())
            .unwrap();
        let (_, _, access, state) = inputs.next().unwrap();
        assert_eq!(access, Access::Unknown);
        assert!(state.is_some_and(|(_, normal)| normal));
    }
}

#[test]
pub(crate) fn declared_access_view_preserves_stopped_and_unvisited_parameter_modes() {
    let (mut checker, program, mut reports, facts) =
        context("d:@\"debug\";f:(a<int32>,p<&!int32>){};n:=1;f(d.panic(\"stop\"),&!n)");
    reports.effects.clear();
    reports.arguments.clear();
    let (view, _) = checker
        .call_snapshots(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let site = *view.calls.first_key_value().unwrap().0;
    let (_, mut inputs) = checker
        .call_mutation_inputs(&view, site, Span::default())
        .unwrap();
    let (_, _, access, state) = inputs.next().unwrap();
    assert_eq!(access, Access::Value);
    assert!(state.is_some_and(|(_, normal)| !normal));
    let (_, _, access, state) = inputs.next().unwrap();
    assert!(matches!(access, Access::Exclusive(_)) && state.is_none());
    assert!(inputs.next().is_none());
}
