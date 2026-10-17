use super::{super::entries::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn signature_input_shapes_reject_changed_captured_and_reported_descriptors() {
    for fault in 0..10 {
        let (mut checker, program) = checked("f:(a<int32>,b<&int32>){};n:1;f(2,&n)");
        let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
        let function = &program.functions[0];
        let local = function.params[1];
        let header = reports.signatures.functions.get_mut(&function.id).unwrap();
        match fault {
            0 => {
                header.inputs.pop();
            }
            1 => header.inputs.push(Shape::Other),
            2 => header.inputs.swap(0, 1),
            3 => header.inputs[1] = Shape::Scalar(ScalarKind::Bool),
            4 => {
                checker.parameters.remove(&local);
            }
            5 => checker.parameters.get_mut(&local).unwrap().function += 1,
            6 => checker.parameters.get_mut(&local).unwrap().position = 0,
            7 => {
                checker.parameters.get_mut(&local).unwrap().shape =
                    Shape::Reference(hir::ReferenceMode::Exclusive)
            }
            8 => {
                let shape = Shape::SharedScalar(ScalarKind::Int {
                    bits: 7,
                    signed: true,
                });
                header.inputs[1] = shape;
                checker.parameters.get_mut(&local).unwrap().shape = shape;
            }
            9 => {
                checker
                    .parameters
                    .insert(usize::MAX, checker.parameters[&local]);
            }
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.parameters,
            checker.edge_counts()
        );
        let error = checker
            .validate_function_signatures(&reports, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
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
pub(crate) fn signature_input_shapes_reject_stale_program_types_before_initializer_access() {
    let (mut checker, mut program) = checked("f:(a<int32>,unused<&int32>){}");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let local = program.functions[0].params[1];
    program.locals[local] = hir::Type::Reference(Box::new(hir::Type::Bool));
    let before = format!("{program:?}{reports:?}{:?}", checker.parameters);
    for result in [
        checker
            .function_signatures(&program, &reports, Span::default())
            .map(|_| ()),
        checker
            .program_signatures(&program, &reports, Span::default())
            .map(|_| ()),
    ] {
        assert!(result.unwrap_err().message.contains("identity"));
    }
    assert_eq!(
        format!("{program:?}{reports:?}{:?}", checker.parameters),
        before
    );
}

#[test]
pub(crate) fn signature_input_shapes_reject_oversized_vectors_before_type_inspection() {
    let (mut checker, program) = checked("f:(a<int32>){}");
    let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
    reports.signatures.functions.get_mut(&0).unwrap().inputs = vec![Shape::Other; MAX_ITEMS + 1];
    let work = checker.flow.work;
    assert!(
        checker
            .checked_function_signature(&reports, 0, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.flow.work - work < 100);
}
