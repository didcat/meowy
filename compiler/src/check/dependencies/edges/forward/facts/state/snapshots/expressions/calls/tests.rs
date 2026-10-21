use super::super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_call_values_use_full_declared_domains_and_fresh_origins() {
    for source in [
        "f<uint8>:(n<uint8>){->n};f(5);f(6)",
        "f<boolean>:(){->true};f();f()",
        "f:(){};f();f()",
        "f<int32>:(n<int32>){->f(n)};f(5)",
        "d:@\"debug\";f<boolean>:(){d.panic(\"stop\")};f()",
    ] {
        let (mut checker, program, _, snapshots) = capture(source);
        for call in checker.invocations.values() {
            let value = snapshots.expressions[&call.point].value.as_ref().unwrap();
            let declared = Domain::declared(
                &program.functions[call.function].result,
                &mut checker.flow,
                Span::default(),
            )
            .unwrap()
            .unwrap();
            assert_eq!(value.domain, declared, "{source}");
            assert_eq!((value.local, value.version), (None, None));
            assert_eq!(
                value.origin,
                Some(Origin::Expression(call.owner, call.point))
            );
            assert!(!value.control);
        }
    }
}

#[test]
pub(crate) fn scalar_call_values_preserve_copies_without_correlating_separate_calls() {
    let (checker, _, _, snapshots) =
        capture("f<uint8>:(){->5};n:=((f())~<uint8>);old:n;n=f();old;n;f()");
    let calls = checker
        .invocations
        .values()
        .map(|call| call.point)
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 3);
    for (index, (&point, _)) in checker.local_reads.iter().enumerate() {
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        let source = snapshots.expressions[&calls[usize::from(index == 2)]]
            .value
            .as_ref()
            .unwrap();
        assert_eq!(value.domain, source.domain);
        assert_eq!(value.origin, source.origin);
        assert!(value.local.is_some() && value.version.is_some());
    }
    let origins = calls
        .iter()
        .map(|point| snapshots.expressions[point].value.as_ref().unwrap().origin)
        .collect::<Vec<_>>();
    assert_ne!(origins[0], origins[1]);
    assert_ne!(origins[1], origins[2]);
}

#[test]
pub(crate) fn scalar_call_values_keep_unknown_arguments_and_opaque_results_unavailable() {
    for source in [
        "f<boolean>:(n<float64>){->true};f(1.5)",
        "f<boolean>:(n<&int32>){->true};n:1;f(&n)",
        "f<boolean>:(n<int32>){->true};f(1+2)",
        "f<float64>:(){->1.5};f()",
        "f<string>:(){->\"cat\"};f()",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let call = checker.invocations.values().next().unwrap();
        assert!(
            snapshots.expressions[&call.point].value.is_none(),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn scalar_call_values_preserve_bottom_and_stopped_argument_boundaries() {
    let (checker, _, _, snapshots) = capture("f<boolean>:(){->true};g:(n<never>){f()}");
    let call = checker.invocations.values().next().unwrap();
    let value = snapshots.expressions[&call.point].value.as_ref().unwrap();
    assert_eq!(value.domain, Domain::BOTTOM);
    assert_eq!(
        (value.local, value.version, value.origin),
        (None, None, None)
    );
    let (checker, _, _, snapshots) = capture(
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};f<boolean>:(n<int32>){->true};f(stop())",
    );
    for call in checker.invocations.values() {
        assert!(!snapshots.expressions.contains_key(&call.point));
        assert!(!snapshots.rows.contains_key(&Port::Normal(call.point)));
    }
}

#[test]
pub(crate) fn scalar_call_values_combine_argument_and_entry_dependencies_before_guard_use() {
    use super::super::super::guards::Authority;
    for (inputs, entry, expected) in [
        ([Some(false), Some(false)], Some(false), Some(false)),
        ([Some(true), Some(false)], Some(false), Some(true)),
        ([None, Some(true)], Some(false), Some(true)),
        ([None, Some(false)], Some(false), None),
        ([Some(false), Some(false)], None, None),
        ([None, None], Some(true), Some(true)),
    ] {
        let (mut checker, _, reports, mut snapshots) =
            capture("f<boolean>:(a<boolean>,b<boolean>){->a};|f(false,true)|1");
        let call = checker.invocations.values().next().unwrap();
        for (arg, input) in call.args.iter().zip(inputs) {
            let fact = snapshots.expressions.get_mut(arg).unwrap();
            if let Some(control) = input {
                fact.value.as_mut().unwrap().control = control;
            } else {
                fact.value = None;
            }
        }
        snapshots.expressions.remove(&call.point).unwrap();
        snapshots
            .call_expression(
                call.point,
                &reports.effects[&call.point],
                entry,
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        let value = snapshots.expressions[&call.point].value.as_ref();
        assert_eq!(value.map(|value| value.control), expected);
        let (&point, guard) = snapshots.guards.iter().next().unwrap();
        let input = guard.input;
        snapshots.guards.remove(&point);
        snapshots
            .guard(
                Port::Normal(point),
                &input,
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert_eq!(
            snapshots.guards[&point].authority,
            match expected {
                Some(true) => Authority::Dependent,
                Some(false) => Authority::Boolean(3),
                None => Authority::Unknown,
            }
        );
    }
}
