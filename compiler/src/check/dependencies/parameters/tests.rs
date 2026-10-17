use super::*;

pub(super) fn seed() -> Checker {
    let mut checker = Checker::new();
    checker.functions.extend([None, None]);
    checker.local(hir::Type::Bool);
    checker.local(hir::Type::Reference(Box::new(hir::Type::Int {
        bits: 32,
        signed: true,
    })));
    checker.local(hir::Type::Null);
    checker.owner = 1;
    checker
}

#[test]
pub(crate) fn parameter_shape_capture_keeps_nested_forward_unused_and_transferred_identities() {
    for source in [
        "f:(a<&int32>){g:(b<&!int32>){} };unused:(r<{n<int32>}>){}",
        "f<(uint8)->null>;f<null>:(n<uint8>){};f(1)",
        "n:2;f:(n<int32>,items<int32[n]>){}",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        let body = checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        let program = hir::Program {
            body,
            functions: std::mem::take(&mut checker.functions)
                .into_iter()
                .flatten()
                .collect(),
            locals: std::mem::take(&mut checker.locals),
        };
        assert!(checker.locals.is_empty());
        assert_eq!(
            checker.parameters.len(),
            program.functions.iter().map(|f| f.params.len()).sum()
        );
        for function in &program.functions {
            for (position, &local) in function.params.iter().enumerate() {
                assert_eq!(
                    checker.parameters[&local],
                    Parameter {
                        function: function.id,
                        position,
                        shape: Completion::of(&program.locals[local]).result
                    }
                );
            }
        }
        let before = checker.parameters.clone();
        checker.entry_reports(&program, Span::default()).unwrap();
        assert_eq!(checker.parameters, before);
    }
}

#[test]
pub(crate) fn parameter_shape_capture_rejects_late_identity_faults_without_partial_publication() {
    for fault in 0..6 {
        let mut checker = seed();
        checker.owner = 2;
        checker
            .capture_parameters(1, &[2], Span::default())
            .unwrap();
        checker.owner = 1;
        let mut ids = vec![0, 1];
        let mut function = 0;
        match fault {
            0 => ids[1] = usize::MAX,
            1 => ids[1] = 0,
            2 => function = usize::MAX,
            3 => checker.owner = 2,
            4 => {
                checker.proofs.bindings.remove(&1);
            }
            5 => ids[1] = 2,
            _ => unreachable!(),
        }
        let before = (checker.parameters.clone(), checker.edge_counts());
        let error = checker
            .capture_parameters(function, &ids, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!((checker.parameters.clone(), checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn parameter_shape_capture_bounds_maps_and_exact_work_before_publication() {
    let mut checker = seed();
    let start = checker.flow.work;
    checker
        .capture_parameters_limited(0, &[0, 1], Span::default(), 2)
        .unwrap();
    let work = checker.flow.work - start;
    let expected = checker.parameters.clone();
    for short in [0, 1] {
        let mut checker = seed();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.capture_parameters_limited(0, &[0, 1], Span::default(), 2);
        if short == 0 {
            result.unwrap();
            assert_eq!(checker.parameters, expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
            assert!(checker.parameters.is_empty());
        }
    }
    let mut checker = seed();
    assert!(
        checker
            .capture_parameters_limited(0, &[0, 1], Span::default(), 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.parameters.is_empty());
    let start = checker.flow.work;
    assert!(
        checker
            .capture_parameters(0, &vec![0; MAX_ITEMS + 1], Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.flow.work, start);
}
