use super::*;
use crate::check::dependencies::edges::forward::arguments::mutations::tests::context;

#[test]
pub(crate) fn call_mutation_targets_borrow_local_alias_and_reference_free_inputs() {
    let source = "f:(p<&!int32>,n<int32>){};n:=1;f(&!n,2);r:{->n:=3;f(&!n,4)};shared:(p<&int32>){};shared(&!n)";
    let (mut checker, program, reports, facts) = context(source);
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(calls.len(), 3);
    for (site, call) in calls {
        assert!(std::ptr::eq(
            call.snapshot.unwrap(),
            &facts.call_inputs[&site]
        ));
        for (position, ((param, point, access, state), targets)) in call.inputs.iter().enumerate() {
            assert_eq!(
                *param,
                program.functions[call.snapshot.unwrap().header.function].params[position]
            );
            assert_eq!(*point, checker.invocations[&site].args[position]);
            assert!(std::ptr::eq(
                state.unwrap().0,
                &facts.call_inputs[&site].args[position].1
            ));
            assert!(!targets.unknown);
            assert_eq!(
                targets.origins.len(),
                usize::from(matches!(access, Access::Exclusive(_)))
            );
            for (origin, id) in &targets.origins {
                assert!(std::ptr::eq(*origin, &state.unwrap().0.origins[0]));
                assert_eq!(checker.place_borrows[id].owner, call.owner);
            }
        }
    }
}

#[test]
pub(crate) fn call_mutation_targets_preserve_stopped_missing_and_opaque_arguments() {
    let source = "d:@\"debug\";f:(a<&!int32>,b<int32>,c<&!int32>){};g:(){};n:=1;m:=2;f(&!n,d.panic(\"stop\"),&!m);g()";
    let (mut checker, program, mut reports, facts) = context(source);
    reports.effects.clear();
    reports.arguments.clear();
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    for call in calls.values() {
        if let Some(snapshot) = call.snapshot {
            assert_eq!(snapshot.normal, 1);
            assert_eq!(snapshot.header.entered, crate::flow::FALSE);
            assert_eq!(call.inputs[0].1.origins.len(), 1);
            assert!(!call.inputs[0].1.unknown);
            assert!(call.inputs[1].0.3.is_some_and(|(_, normal)| !normal));
            assert!(call.inputs[1].1.unknown && call.inputs[1].1.origins.is_empty());
            assert!(call.inputs[2].0.3.is_none() && call.inputs[2].1.unknown);
        } else {
            assert!(call.inputs.is_empty());
        }
    }
    let (mut checker, program, reports, facts) =
        context("f:(p<&!int32>){};g:(p<&!int32>){f(p)};h:(r<{p<&int32>}>){};n:1;h({->p:&n})");
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    for call in calls.values() {
        assert!(call.inputs[0].1.unknown && call.inputs[0].1.origins.is_empty());
    }
}

#[test]
pub(crate) fn call_mutation_targets_bound_combined_payload_and_late_work() {
    let (mut checker, program, reports, facts) =
        context("f:(p<&!int32>){};n:=1;f(&!n);r:{->n:=2;f(&!n)}");
    let start = checker.flow.work;
    let (_, left) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let parts = MAX_EDGES - left;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (limit, spare, pass) in [(parts - 1, 0, false), (parts, 1, false), (parts, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + spare;
        checker.flow.full = false;
        assert_eq!(
            checker
                .call_mutation_targets(&program, &reports, &facts, Span::default(), limit)
                .is_ok(),
            pass
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn call_mutation_targets_keep_opaque_storage_and_allocator_bounds_conservative() {
    let (mut checker, program, reports, facts) =
        context("m:@\"memory\";f:(p<&!int32>){};r:{->h:m.heap;->n:=1};f(&!(r.n))");
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let targets = &calls.first_key_value().unwrap().1.inputs[0].1;
    assert!(targets.unknown && targets.origins.is_empty());
}
