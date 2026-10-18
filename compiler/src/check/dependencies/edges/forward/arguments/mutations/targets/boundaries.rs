use super::*;
use crate::borrow_value::{Source, Step};
use crate::check::dependencies::edges::forward::arguments::mutations::tests::context;

#[test]
pub(crate) fn call_mutation_targets_keep_guarded_alternatives_without_specialization() {
    let source = "f:(p<&!int32>){};g:(flag<boolean>){a:=1;b:=2;p:=&!a;|flag|p=&!b;f(p)}";
    let (mut checker, program, reports, facts) = context(source);
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let call = calls.first_key_value().unwrap().1;
    assert_ne!(call.owner, 0);
    let targets = &call.inputs[0].1;
    assert!(!targets.unknown);
    assert_eq!(targets.origins.len(), 2);
    let first = targets.origins[0].0;
    let second = targets.origins[1].0;
    assert_ne!(first.source, second.source);
    assert!(!checker.flow.overlap(first.guard, second.guard));
    for ((origin, _), original) in targets
        .origins
        .iter()
        .zip(&call.snapshot.unwrap().args[0].1.origins)
    {
        assert!(std::ptr::eq(*origin, original));
    }
}

#[test]
pub(crate) fn call_mutation_targets_preserve_inputs_before_later_argument_retargeting() {
    let source = "f:(p<&!int32>,n<int32>){};a:=1;b:=2;p:=&!a;f(p,{p=&!b;->0});f(p,0)";
    let (mut checker, program, reports, facts) = context(source);
    let first = checker
        .place_borrows
        .first_key_value()
        .unwrap()
        .1
        .place
        .root;
    let last = checker.place_borrows.last_key_value().unwrap().1.place.root;
    assert_ne!(first, last);
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let targets = &calls.first_key_value().unwrap().1.inputs[0].1;
    assert!(!targets.unknown && targets.origins.len() == 1);
    assert_eq!(
        targets.origins[0].0.source,
        Source::Local {
            id: first,
            fields: vec![]
        }
    );
    let later = &calls.last_key_value().unwrap().1.inputs[0].1;
    assert!(!later.unknown && later.origins.len() == 1);
    assert_eq!(
        later.origins[0].0.source,
        Source::Local {
            id: last,
            fields: vec![]
        }
    );
}

#[test]
pub(crate) fn call_mutation_targets_keep_missing_components_bounds_and_guard_gaps_unknown() {
    for fault in 0..6 {
        let (mut checker, program, reports, mut facts) = context("f:(p<&!int32>){};n:=1;f(&!n)");
        let state = &mut facts.call_inputs.first_entry().unwrap().into_mut().args[0].1;
        match fault {
            0 => state.origins.clear(),
            1 => state.origins[0].component.push(Step::Deref),
            2 => state.origins[0].component.push(Step::Slot(1)),
            3 => {
                state.origins[0].source = Source::Local {
                    id: usize::MAX,
                    fields: vec![],
                }
            }
            4 => state.bounds = state.origins.clone(),
            5 => state.origins[0].guard = crate::flow::FALSE,
            _ => unreachable!(),
        }
        let (calls, _) = checker
            .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let targets = &calls.first_key_value().unwrap().1.inputs[0].1;
        assert!(targets.unknown, "fault {fault}");
        assert_eq!(targets.origins.len(), usize::from(fault >= 4));
    }
}

#[test]
pub(crate) fn call_mutation_targets_reject_late_cross_kind_origins_without_partial_output() {
    let (mut checker, program, reports, mut facts) =
        context("f:(p<&!int32>){};g:(p<&!boolean>){};n:=1;b:=false;f(&!n);g(&!b)");
    let source = facts.call_inputs.first_key_value().unwrap().1.args[0]
        .1
        .origins[0]
        .source
        .clone();
    facts.call_inputs.last_entry().unwrap().get_mut().args[0]
        .1
        .origins[0]
        .source = source;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let Err(error) =
        checker.call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
    else {
        panic!()
    };
    assert!(error.message.contains("identity"));
    assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
}
