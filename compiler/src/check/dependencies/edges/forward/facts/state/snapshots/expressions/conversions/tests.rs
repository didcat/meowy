use super::super::super::capture::tests::capture;
use super::super::*;

#[test]
pub(crate) fn nullable_conversion_values_preserve_domains_origins_and_copies_across_writes() {
    for (ty, first, next) in [
        ("int32", "7", "9"),
        ("boolean", "true", "false"),
        ("uint64", "null", "null"),
    ] {
        let source = format!("n:={first};a<{ty}><null>:n;copy:a;n={next};a;copy;n");
        let (checker, _, _, snapshots) = capture(&source);
        let (&point, op) = checker
            .coercions
            .iter()
            .find(|(_, op)| op.scalar.is_some())
            .unwrap();
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        let input = snapshots.expressions[&op.input].value.as_ref().unwrap();
        assert_eq!(value, input);
        assert_eq!(value.domain.null, first == "null");
        assert!(value.origin.is_some());
        let mut count = 0;
        for &read in checker.local_reads.keys() {
            let span = checker.points[read].span;
            if span.start > source.rfind(";a;copy;n").unwrap() {
                let copied = snapshots.expressions[&read].value.as_ref().unwrap();
                if &source[span.start..span.end] == "n" {
                    assert_ne!(copied.origin, value.origin);
                } else {
                    assert_eq!(copied.domain, value.domain);
                    assert_eq!(copied.origin, value.origin);
                }
                count += 1;
            }
        }
        assert_eq!(count, 3);
    }
}

#[test]
pub(crate) fn nullable_conversion_values_reach_guarded_block_slots_and_keep_emitted_history() {
    for (source, expected) in [
        ("v<boolean><null>:{|true|->true};v", Domain::boolean(true)),
        ("v<int8><null>:{|false|->7};v", Domain::NULL),
        (
            "f:(flag<boolean>){n:=7;v<int32><null>:{|flag|->n;n=9};v}",
            Domain::integer(7, &mut Flow::new(), Span::default())
                .unwrap()
                .join(&Domain::NULL, &mut Flow::new(), Span::default())
                .unwrap(),
        ),
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let target = &checker.emissions.values().next().unwrap().targets[0];
        let point = checker.bodies[&target.block].parent.unwrap();
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        assert_eq!(value.domain, expected, "{source}");
        assert!(!value.control);
        if source.starts_with("f:") {
            let emitted = snapshots.emissions[&target.id].value.as_ref().unwrap();
            assert!(!emitted.domain.null);
            assert!(emitted.origin.is_some());
            assert_eq!(value.origin, None);
            let Some(View::Known(state)) = snapshots.after(Port::BlockResult(target.block)) else {
                panic!()
            };
            assert_eq!(
                state.slot(target.block).unwrap().data,
                Data::Conditional {
                    domain: emitted.domain.clone(),
                    origin: emitted.origin,
                }
            );
        }
    }
}

#[test]
pub(crate) fn nullable_conversion_values_feed_calls_without_specializing_declared_results() {
    let source = "take<uint8>:(n<uint8><null>){->3};v<uint8><null>:{->take(7)};v";
    let (mut checker, _, _, snapshots) = capture(source);
    let call = checker.invocations.values().next().unwrap();
    let argument = snapshots.expressions[&call.args[0]].value.as_ref().unwrap();
    assert_eq!(
        argument.domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    let result = snapshots.expressions[&call.point].value.as_ref().unwrap();
    assert_eq!(
        result.domain,
        Domain::scalar(
            crate::check::dependencies::ScalarKind::Int {
                bits: 8,
                signed: false
            },
            &mut checker.flow,
            Span::default()
        )
        .unwrap()
        .unwrap()
    );
    for (&point, op) in checker
        .coercions
        .iter()
        .filter(|(_, op)| op.scalar.is_some())
    {
        assert_eq!(
            snapshots.expressions[&point],
            snapshots.expressions[&op.input]
        );
    }
    let body = checker
        .bodies
        .values()
        .find(|body| body.parent.is_some())
        .unwrap();
    let value = snapshots.expressions[&body.parent.unwrap()]
        .value
        .as_ref()
        .unwrap();
    assert_eq!(value.domain, result.domain);
    assert_eq!(value.origin, result.origin);
}
