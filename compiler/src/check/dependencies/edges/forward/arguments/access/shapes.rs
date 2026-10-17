use super::{super::tests::context, *};
use crate::check::dependencies::{ScalarKind, bodies::completion::Shape};

#[test]
pub(crate) fn call_parameter_shapes_keep_declared_domains_across_different_arguments() {
    let source = "f:(n<int32><null>){};f(1);f(null)";
    crate::compile(source).unwrap();
    let (mut checker, reports) = context(source);
    let calls = checker.invocations.values().cloned().collect::<Vec<_>>();
    let (forest, _) = checker
        .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    for expected in calls {
        let (call, inputs) = checker
            .call_argument_forest(&forest, expected.site, Span::default())
            .unwrap();
        let header = &reports.signatures.functions[&call.function];
        assert_eq!(header.inputs, [Shape::Union { members: 2 }]);
        for (position, (param, point, row, marker)) in inputs.enumerate() {
            assert_eq!(param, header.params[position]);
            assert_eq!(point, call.args[position]);
            assert!(row.unwrap().normal && marker.is_some());
            assert!(!reports.initializers.contains_key(&param));
        }
    }
}

#[test]
pub(crate) fn call_parameter_shapes_preserve_exclusive_modes_for_stopped_inputs() {
    let source = "d:@\"debug\";f:(a<int32>,p<&!int32>){};n:=1;f(d.panic(\"stop\"),&!n)";
    crate::compile(source).unwrap();
    let (mut checker, reports) = context(source);
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    let (forest, _) = checker
        .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    let (_, inputs) = checker
        .call_argument_forest(&forest, call.site, Span::default())
        .unwrap();
    assert!(!reports.effects.contains_key(&call.point));
    assert_eq!(
        reports.signatures.functions[&call.function].inputs[1],
        Shape::Reference(hir::ReferenceMode::Exclusive)
    );
    for (position, (_, _, row, marker)) in inputs.enumerate() {
        assert_eq!(row.is_some(), position == 0);
        assert!(marker.is_none());
        assert!(!row.is_some_and(|row| row.normal));
    }
}

#[test]
pub(crate) fn call_parameter_shapes_keep_aggregate_interiors_opaque() {
    let source =
        "f:(plain<{n<int32>}>,borrowed<{p<&int32>}>,list<&int32[2]>,record<&{n<int32>}>){}";
    crate::compile(source).unwrap();
    let (_, reports) = context(source);
    let header = reports.signatures.functions.first_key_value().unwrap().1;
    assert_eq!(
        header.inputs,
        [
            Shape::Record { fields: 1 },
            Shape::Record { fields: 1 },
            Shape::Reference(hir::ReferenceMode::Shared),
            Shape::Reference(hir::ReferenceMode::Shared)
        ]
    );
    assert!(reports.argument_sources.is_empty() && reports.argument_walk.visits.is_empty());
    for param in &header.params {
        assert!(!reports.initializers.contains_key(param));
    }
}

#[test]
pub(crate) fn call_parameter_shapes_revalidate_unused_capture_before_exposing_pairs() {
    for fault in 0..4 {
        let (mut checker, reports) = context("f:(a<int32>,unused<&int32>){};n:1;f(2,&n)");
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let local = reports.signatures.functions[&call.function].params[1];
        let (forest, _) = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        let param = checker.parameters.get_mut(&local).unwrap();
        match fault {
            0 => param.shape = Shape::SharedScalar(ScalarKind::Bool),
            1 => param.position = 0,
            2 => param.function += 1,
            3 => {
                param.shape = Shape::SharedScalar(ScalarKind::Int {
                    bits: 32,
                    signed: false,
                })
            }
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.parameters,
            checker.edge_counts()
        );
        let Err(error) = checker.call_argument_forest(&forest, call.site, Span::default()) else {
            panic!("fault {fault}")
        };
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("function-signature identity"));
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.parameters,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn call_parameter_shapes_reject_unmatched_capture_for_empty_signatures() {
    for oversized in [false, true] {
        let (mut checker, reports) = context("f:(){};f()");
        let site = *checker.invocations.first_key_value().unwrap().0;
        let (forest, _) = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        let param = crate::check::dependencies::Parameter {
            function: 0,
            position: 0,
            shape: Shape::Scalar(ScalarKind::Bool),
        };
        let count = if oversized { MAX_EDGES + 1 } else { 1 };
        checker.parameters = (0..count).map(|local| (local, param)).collect();
        let error = checker
            .checked_function_signature(&reports, 0, Span::default())
            .unwrap_err();
        assert!(
            error
                .message
                .contains(if oversized { "budget" } else { "identity" })
        );
        let Err(error) = checker.call_argument_forest(&forest, site, Span::default()) else {
            panic!()
        };
        assert!(
            error
                .message
                .contains(if oversized { "budget" } else { "identity" })
        );
    }
}

#[test]
pub(crate) fn call_parameter_shapes_reject_extra_capture_for_empty_programs() {
    let (mut checker, program) =
        crate::check::dependencies::edges::forward::entries::tests::checked("");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    checker.parameters.insert(
        0,
        crate::check::dependencies::Parameter {
            function: 0,
            position: 0,
            shape: Shape::Scalar(ScalarKind::Bool),
        },
    );
    assert!(
        checker
            .program_signatures(&program, &reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
}
