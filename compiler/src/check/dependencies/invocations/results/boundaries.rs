use super::*;
use crate::check::dependencies::bodies::completion::MAX_BODY_RESULTS;

#[test]
pub(crate) fn call_result_capture_rejects_invalid_shapes_and_stale_return_boundaries() {
    for retained in [false, true] {
        let mut checker = check("f<int32>:(n<int32>){->n};f(1)");
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        if !retained {
            checker.invocations.clear();
            checker.invocation_edges = 0;
        }
        let before = format!("{:?}{:?}", checker.invocations, checker.edge_counts());
        for shape in [
            Shape::Never,
            Shape::Scalar(ScalarKind::Int {
                bits: 7,
                signed: true,
            }),
            Shape::Scalar(ScalarKind::Float { bits: 16 }),
            Shape::SharedScalar(ScalarKind::Int {
                bits: 128,
                signed: false,
            }),
            Shape::SharedScalar(ScalarKind::Float { bits: 16 }),
            Shape::Record {
                fields: MAX_BODY_RESULTS + 1,
            },
            Shape::List {
                capacity: crate::list::MAX_CAPACITY + 1,
            },
            Shape::Union { members: 1 },
            Shape::Union {
                members: MAX_BODY_RESULTS + 1,
            },
        ] {
            let mut changed = call.clone();
            changed.shape = shape;
            let error = checker.invocation(changed).unwrap_err();
            assert_eq!(error.code, "B001");
            assert!(error.message.contains("call-input identity"));
            assert_eq!(
                format!("{:?}{:?}", checker.invocations, checker.edge_counts()),
                before
            );
        }
        let mut changed = call.clone();
        changed.may_return = false;
        assert!(
            checker
                .invocation(changed)
                .unwrap_err()
                .message
                .contains("identity")
        );
        if retained {
            let mut changed = call;
            changed.shape = Shape::Scalar(ScalarKind::Bool);
            assert!(changed.valid_result());
            assert!(
                checker
                    .invocation(changed)
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
        }
        assert_eq!(
            format!("{:?}{:?}", checker.invocations, checker.edge_counts()),
            before
        );
    }
    let mut checker = check("f<never>:(){'loop{'loop.restart()}};f()");
    let mut call = checker.invocations.first_key_value().unwrap().1.clone();
    call.may_return = true;
    assert!(!call.valid_result());
    assert!(
        checker
            .invocation(call)
            .unwrap_err()
            .message
            .contains("identity")
    );
}

#[test]
pub(crate) fn call_result_capture_charges_shape_validation_before_atomic_publication() {
    let mut checker = check("f<int32[3]>:(n<int32>){->[n]};f(1)");
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    checker.invocations.clear();
    checker.invocation_edges = 0;
    let before = checker.edge_counts();
    let start = checker.flow.work;
    checker.invocation(call.clone()).unwrap();
    let work = checker.flow.work - start;
    let expected = checker.invocations.clone();
    let counts = checker.edge_counts();
    for short in [0, 1] {
        checker.invocations.clear();
        checker.invocation_edges = 0;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.invocation(call.clone());
        if short == 0 {
            result.unwrap();
            assert_eq!(checker.invocations, expected);
            assert_eq!(checker.edge_counts(), counts);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
            assert!(checker.invocations.is_empty());
            assert_eq!(checker.edge_counts(), before);
        }
    }
}
