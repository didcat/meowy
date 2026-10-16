use super::*;

pub(super) fn checked() -> (Checker, hir::Program, Reports) {
    let ast =
        crate::parser::parse("x:1;f<int32>:(a<int32>,b<int32>){->a+b};g<int32>:(c<int32>){->c}")
            .unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let program = hir::Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, ast.span).unwrap();
    (checker, program, reports)
}

#[test]
pub(crate) fn program_signatures_follow_function_ids_in_reversed_order() {
    let (mut checker, mut program, reports) = checked();
    let expected = program
        .functions
        .iter()
        .flat_map(|function| function.params.iter().copied())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected.len(), 3);
    program.functions.reverse();
    assert_eq!(
        checker
            .program_signatures(&program, &reports, Span::default())
            .unwrap()
            .params
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        expected
    );
}

#[test]
pub(crate) fn program_signatures_reject_owner_body_and_local_corruption() {
    for fault in 0..19 {
        let (mut checker, mut program, mut reports) = checked();
        let id = program.functions[0].body.id;
        match fault {
            0 => program.functions[0].id = usize::MAX,
            1 => program.functions[0].id = program.functions[1].id,
            2 => program.functions[0].body.id = program.functions[1].body.id,
            3 => program.functions[0].body.id = usize::MAX,
            4 => checker.bodies.get_mut(&id).unwrap().owner = 0,
            5 => {
                reports
                    .entries
                    .get_mut(&(program.functions[0].id + 1))
                    .unwrap()
                    .0 = program.body.id
            }
            6 => program.functions[0].params[0] = reports.locals,
            7 => program.functions[0].params[0] = program.functions[0].params[1],
            8 => program.functions[0].params[0] = program.functions[1].params[0],
            9 => reports.locals += 1,
            10 => {
                reports.entries.remove(&0);
            }
            11 => program.body.id = id,
            12 => {
                checker.bodies.remove(&id);
            }
            13 => program.functions[0].params.swap(0, 1),
            14 => program.functions[0].result = hir::Type::Bool,
            15 => program.functions[1] = program.functions[0].clone(),
            16 => {
                reports
                    .signatures
                    .params
                    .remove(&program.functions[0].params[0]);
            }
            17 => {
                reports
                    .signatures
                    .params
                    .get_mut(&program.functions[0].params[0])
                    .unwrap()
                    .1 = 1;
            }
            18 => {
                reports
                    .signatures
                    .params
                    .insert(0, (program.functions[0].id, 2));
            }
            _ => unreachable!(),
        }
        let before = format!("{program:?}{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .program_signatures(&program, &reports, Span::default())
                .unwrap_err()
                .message
                .contains("function-signature identity")
        );
        assert!(
            checker
                .binding_initializers(&program, &reports, Span::default())
                .is_err(),
            "ordinary fault {fault}"
        );
        assert!(
            checker
                .shared_initializers(&program, &reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .is_err(),
            "shared fault {fault}"
        );
        assert_eq!(
            format!("{program:?}{reports:?}{:?}", checker.edge_counts()),
            before
        );
    }
}

#[test]
pub(crate) fn program_signatures_charge_exact_shared_work() {
    let (mut checker, program, reports) = checked();
    let before = checker.flow.work;
    let expected = checker
        .program_signatures(&program, &reports, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.program_signatures(&program, &reports, Span::default());
        if spare == 0 {
            assert_eq!(result.unwrap(), expected);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(
                result
                    .unwrap_err()
                    .message
                    .contains("function-signature budget")
            );
            assert!(checker.flow.exceeded());
        }
    }
}
