use super::super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn nullable_block_values_preserve_emissions_and_copy_identity_across_tail_writes() {
    for (first, later, domain) in [
        ("true", "false", Domain::boolean(true)),
        (
            "7",
            "9",
            Domain::integer(7, &mut Flow::new(), Span::default()).unwrap(),
        ),
    ] {
        let source = format!(
            "f:(flag<boolean>){{n:={first};v:{{|flag|->n;n={later}}};copy:v;n={later};v;copy;n}}"
        );
        let (mut checker, _, _, snapshots) = capture(&source);
        let target = &checker.emissions.values().next().unwrap().targets[0];
        let point = checker.bodies[&target.block].parent.unwrap();
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        let expected = domain
            .join(&Domain::NULL, &mut checker.flow, Span::default())
            .unwrap();
        assert_eq!(value.domain, expected);
        assert_eq!(
            (value.local, value.version, value.origin),
            (None, None, None)
        );
        assert!(!value.control);
        let emitted = snapshots.emissions[&target.id].value.as_ref().unwrap();
        assert_eq!(emitted.domain, domain);
        assert!(emitted.origin.is_some());
        let Some(View::Known(state)) = snapshots.after(Port::BlockResult(target.block)) else {
            panic!()
        };
        assert_eq!(
            state.slot(target.block).unwrap().data,
            Data::Conditional {
                domain: domain.clone(),
                origin: emitted.origin,
            }
        );
        let mut copies = Vec::new();
        for &point in checker.local_reads.keys() {
            let span = checker.points[point].span;
            if span.start > source.rfind(";v;copy;n").unwrap()
                && &source[span.start..span.end] != "n"
            {
                let copy = snapshots.expressions[&point].value.as_ref().unwrap();
                assert_eq!(copy.domain, expected);
                assert!(copy.origin.is_some());
                assert_ne!(copy.origin, emitted.origin);
                copies.push(copy.origin);
            }
        }
        assert_eq!(copies.len(), 2);
        assert_eq!(copies[0], copies[1]);
    }
}

#[test]
pub(crate) fn nullable_block_values_keep_the_only_live_origin_and_materialize_omitted_null() {
    for (ty, literal) in [("boolean", "true"), ("int8", "7")] {
        for flag in ["true", "false"] {
            let source = format!("n<{ty}>:{literal};flag:={flag};v:{{|flag|->n}};v");
            let (checker, _, _, snapshots) = capture(&source);
            let target = &checker.emissions.values().next().unwrap().targets[0];
            let body = &checker.bodies[&target.block];
            assert!(body.scalar.unwrap().nullable);
            let point = body.parent.unwrap();
            let value = snapshots.expressions[&point].value.as_ref().unwrap();
            let Some(View::Known(state)) = snapshots.after(Port::BlockResult(target.block)) else {
                panic!()
            };
            if flag == "true" {
                let emitted = snapshots.emissions[&target.id].value.as_ref().unwrap();
                assert_eq!(value.domain, emitted.domain);
                assert!(!value.domain.null);
                assert_eq!(value.origin, emitted.origin);
                assert!(matches!(
                    state.slot(target.block).unwrap().data,
                    Data::Initialized { .. }
                ));
            } else {
                assert_eq!(value.domain, Domain::NULL);
                assert_eq!(value.origin, Some(Origin::Expression(body.owner, point)));
                assert_eq!(state.slot(target.block).unwrap().data, Data::Uninitialized);
            }
            assert!(!value.control);
        }
    }
}

#[test]
pub(crate) fn nullable_block_values_join_nested_leaves_and_multiple_emissions() {
    let source = "f:(a<boolean>,b<boolean>){v:'out{|a|{'out->7;'out.leave()};|b|'out->9};v}";
    let (mut checker, _, _, snapshots) = capture(source);
    let target = &checker.emissions.values().next().unwrap().targets[0];
    let point = checker.bodies[&target.block].parent.unwrap();
    let value = snapshots.expressions[&point].value.as_ref().unwrap();
    let first = Domain::integer(7, &mut checker.flow, Span::default()).unwrap();
    let second = Domain::integer(9, &mut checker.flow, Span::default()).unwrap();
    let emitted = first
        .join(&second, &mut checker.flow, Span::default())
        .unwrap();
    let expected = emitted
        .join(&Domain::NULL, &mut checker.flow, Span::default())
        .unwrap();
    assert_eq!(value.domain, expected);
    assert_eq!(value.origin, None);
    let Some(View::Known(state)) = snapshots.after(Port::BlockResult(target.block)) else {
        panic!()
    };
    assert_eq!(
        state.slot(target.block).unwrap().data,
        Data::Conditional {
            domain: emitted,
            origin: None
        }
    );
}

#[test]
pub(crate) fn nullable_block_values_feed_nested_results_and_call_arguments() {
    let source =
        "take<boolean>:(n<int32><null>){->true};f:(flag<boolean>){v:{->{|flag|->7}};take(v)}";
    let (mut checker, _, _, snapshots) = capture(source);
    let expected = Domain::integer(7, &mut checker.flow, Span::default())
        .unwrap()
        .join(&Domain::NULL, &mut checker.flow, Span::default())
        .unwrap();
    let blocks = checker
        .bodies
        .values()
        .filter(|body| body.parent.is_some() && body.scalar.is_some_and(|scalar| scalar.nullable))
        .collect::<Vec<_>>();
    assert_eq!(blocks.len(), 2);
    for body in blocks {
        assert_eq!(
            snapshots.expressions[&body.parent.unwrap()]
                .value
                .as_ref()
                .unwrap()
                .domain,
            expected
        );
    }
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(
        snapshots.expressions[&call.args[0]]
            .value
            .as_ref()
            .unwrap()
            .domain,
        expected
    );
    assert_eq!(
        snapshots.expressions[&call.point]
            .value
            .as_ref()
            .unwrap()
            .domain,
        Domain::scalar(
            crate::check::dependencies::ScalarKind::Bool,
            &mut checker.flow,
            Span::default()
        )
        .unwrap()
        .unwrap()
    );
}
