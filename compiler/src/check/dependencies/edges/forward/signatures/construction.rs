use super::{super::entries::tests::checked, *};

#[test]
pub(crate) fn signature_construction_checks_source_entries_and_parameters_atomically() {
    for fault in 0..20 {
        let (mut checker, mut program) = checked("x:1;f:(a<int32>,b<boolean>){};g:(c<int32>){}");
        let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
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
            13 => checker.bodies.get_mut(&program.body.id).unwrap().parent = Some(id),
            14 => checker.bodies.get_mut(&id).unwrap().parent = Some(program.body.id),
            15 => {
                checker
                    .proofs
                    .bindings
                    .remove(&program.functions[0].params[0]);
            }
            16 => program.locals.push(hir::Type::Bool),
            17 => {
                reports
                    .entries
                    .insert(usize::MAX, (program.body.id, Default::default()));
            }
            18 => {
                program.functions[0].params.clear();
                program.functions[1] = program.functions[0].clone();
            }
            19 => checker.bodies.get_mut(&program.body.id).unwrap().owner = 1,
            _ => unreachable!(),
        }
        let before = format!(
            "{program:?}{reports:?}{:?}{:?}{:?}",
            checker.bodies,
            checker.proofs.bindings,
            checker.edge_counts()
        );
        let error = checker
            .function_signatures(&program, &reports, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert_eq!(
            error.message,
            "proof function-signature identity mismatch is not supported by this bootstrap compiler",
            "fault {fault}"
        );
        assert_eq!(
            format!(
                "{program:?}{reports:?}{:?}{:?}{:?}",
                checker.bodies,
                checker.proofs.bindings,
                checker.edge_counts()
            ),
            before
        );
    }
}
