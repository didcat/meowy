use super::super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_wrapper_histories_survive_unqualified_calls_and_storage_updates() {
    for middle in ["g()", "x:=1;x=2"] {
        let source = format!("g:(){{}};f:(n<uint8>){{((n)~<uint8>);{middle};((n)~<uint8>)}}");
        let (checker, program, _, snapshots) = capture(&source);
        let function = program
            .functions
            .iter()
            .find(|function| function.name == "f")
            .unwrap();
        let mut known = 0;
        let mut missing = 0;
        for (&point, op) in &checker.typed_ops {
            let fact = &snapshots.expressions[&point];
            assert_eq!(fact.owner, function.id + 1);
            assert_eq!(fact, &snapshots.expressions[&op.input]);
            if let Some(value) = &fact.value {
                assert_eq!(value.origin, Some(Origin::Parameter(function.id, 0)));
                assert_eq!(value.version.unwrap().local, function.params[0]);
                known += 1;
            } else {
                missing += 1;
            }
        }
        assert_eq!((known, missing), (1, 1));
    }
}

#[test]
pub(crate) fn scalar_wrapper_histories_own_copied_domains_and_control_marks() {
    let (mut checker, _, _, mut snapshots) = capture("((7))");
    let (&point, group) = checker
        .group_inputs
        .iter()
        .min_by_key(|(_, group)| group.span.start)
        .unwrap();
    let prior = snapshots.expressions[&point].value.clone().unwrap();
    let source = snapshots
        .expressions
        .get_mut(&group.input)
        .unwrap()
        .value
        .as_mut()
        .unwrap();
    source.domain = Domain::integer(9, &mut checker.flow, Span::default()).unwrap();
    source.control = true;
    assert_eq!(snapshots.expressions[&point].value.as_ref(), Some(&prior));
    assert_eq!(
        prior.domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    assert!(!prior.control);
    assert_eq!(prior.version, None);
}
