use super::{tests::checked, *};

#[test]
pub(crate) fn program_signatures_bound_oversized_source_and_retained_parameter_context() {
    for fault in 0..4 {
        let (mut checker, mut program, mut reports) = checked();
        let id = program.functions[0].id;
        match fault {
            0 => program.functions[0].params = vec![0; MAX_ITEMS + 1],
            1 => reports.signatures.functions.get_mut(&id).unwrap().params = vec![0; MAX_ITEMS + 1],
            2 => {
                reports.signatures.params = (0..=MAX_EDGES / PARAM_PARTS)
                    .map(|local| (local, (id, local)))
                    .collect()
            }
            3 => program
                .functions
                .resize(MAX_ENTRIES, program.functions[0].clone()),
            _ => unreachable!(),
        }
        let edges = checker.edge_counts();
        let parts = reports.parts;
        let start = checker.flow.work;
        let error = checker
            .program_signatures(&program, &reports, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("function-signature budget"),
            "fault {fault}"
        );
        assert!(checker.flow.work - start < 100);
        assert!(
            checker
                .binding_initializers(&program, &reports, Span::default())
                .unwrap_err()
                .message
                .contains("function-signature budget")
        );
        assert!(
            checker
                .shared_initializers(&program, &reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap_err()
                .message
                .contains("function-signature budget")
        );
        assert_eq!(checker.edge_counts(), edges);
        assert_eq!(reports.parts, parts);
    }
}

#[test]
pub(crate) fn program_signatures_reject_duplicate_empty_functions_and_coherent_stale_headers() {
    for fault in 0..3 {
        let (mut checker, mut program) =
            super::super::super::entries::tests::checked("x:1;f:(){};g:(){}");
        let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
        let id = program.functions[0].id;
        match fault {
            0 => program.functions[1] = program.functions[0].clone(),
            1 => {
                reports
                    .signatures
                    .functions
                    .get_mut(&id)
                    .unwrap()
                    .params
                    .push(0);
                reports.signatures.params.insert(0, (id, 0));
                let shape = Completion::of(&program.locals[0]).result;
                reports
                    .signatures
                    .functions
                    .get_mut(&id)
                    .unwrap()
                    .inputs
                    .push(shape);
                checker.parameters.insert(
                    0,
                    crate::check::dependencies::Parameter {
                        function: id,
                        position: 0,
                        shape,
                    },
                );
            }
            2 => reports.signatures.functions.get_mut(&id).unwrap().shape = Shape::Never,
            _ => unreachable!(),
        }
        checker
            .validate_function_signatures(&reports, Span::default())
            .unwrap();
        let before = format!("{program:?}{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .program_signatures(&program, &reports, Span::default())
                .unwrap_err()
                .message
                .contains("function-signature identity")
        );
        assert_eq!(
            format!("{program:?}{reports:?}{:?}", checker.edge_counts()),
            before
        );
    }
}
