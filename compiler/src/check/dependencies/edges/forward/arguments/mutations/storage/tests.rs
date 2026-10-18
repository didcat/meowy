use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;

#[test]
pub(crate) fn call_storage_qualifies_exact_scalars_and_final_field_permissions() {
    for (source, kind) in [
        ("n:=true;p:&!n", ScalarKind::Bool),
        (
            "n<uint8>:=7;p:&!n",
            ScalarKind::Int {
                bits: 8,
                signed: false,
            },
        ),
        ("n<float32>:=1.5;p:&!n", ScalarKind::Float { bits: 32 }),
        (
            "r:{->inner:{->n:=1;->tail:false}};p:&!(r.inner.n)",
            ScalarKind::Int {
                bits: 32,
                signed: true,
            },
        ),
        (
            "f:(){n:=1;p:&!n}",
            ScalarKind::Int {
                bits: 32,
                signed: true,
            },
        ),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, program) = checked(source);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let id = *checker.place_borrows.first_key_value().unwrap().0;
        assert_eq!(checker.place_borrows[&id].scalar, Some(kind));
        assert_eq!(
            checker
                .mutation_place(&program, &reports, id, Span::default())
                .unwrap(),
            Some(kind)
        );
    }
}

#[test]
pub(crate) fn call_storage_rejects_stale_types_paths_permissions_and_identities() {
    for fault in 0..11 {
        let (mut checker, mut program) =
            checked("r:{->inner:{->n:=1;->tail:false}};p:&!(r.inner.n)");
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let (&id, op) = checker.place_borrows.first_key_value().unwrap();
        let root = op.place.root;
        let indices = op.place.fields.clone();
        match fault {
            0 => {
                program.locals.pop();
            }
            1 => {
                checker.proofs.bindings.remove(&root);
            }
            2 => checker.place_borrows.get_mut(&id).unwrap().storage = usize::MAX,
            3 => checker.place_borrows.get_mut(&id).unwrap().counts[0] += 1,
            4 => checker.place_borrows.get_mut(&id).unwrap().place.fields[1] = usize::MAX,
            5 => checker.place_borrows.get_mut(&id).unwrap().scalar = Some(ScalarKind::Bool),
            6 => checker.points[id].owner += 1,
            _ => {
                let Type::Record { fields, .. } = &mut program.locals[root] else {
                    panic!()
                };
                let Type::Record { fields, .. } = &mut fields[indices[0]].ty else {
                    panic!()
                };
                match fault {
                    7 => fields[indices[1]].ty = Type::Bool,
                    8 => fields[indices[1]].mutable = false,
                    9 => {
                        fields.pop();
                    }
                    10 => fields[1 - indices[1]].ty = Type::Reference(Box::new(Type::Bool)),
                    _ => unreachable!(),
                }
            }
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let result = checker.mutation_place(&program, &reports, id, Span::default());
        if fault == 10 {
            assert_eq!(result.unwrap(), None);
        } else {
            let error = result.unwrap_err();
            assert!(
                error.message.contains("identity"),
                "fault {fault}: {error:?}"
            );
        }
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn call_storage_keeps_shared_and_alias_places_outside_local_qualification() {
    for source in ["n:1;p:&n", "r:{->n:=1;p:&!n}"] {
        let (mut checker, program) = checked(source);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let id = *checker.place_borrows.first_key_value().unwrap().0;
        assert_eq!(
            checker
                .mutation_place(&program, &reports, id, Span::default())
                .unwrap(),
            None
        );
    }
}

#[test]
pub(crate) fn call_storage_bounds_exact_work_and_rejects_oversized_paths() {
    let (mut checker, program) = checked("r:{->n:=1};p:&!(r.n)");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let id = *checker.place_borrows.first_key_value().unwrap().0;
    let start = checker.flow.work;
    checker
        .mutation_place(&program, &reports, id, Span::default())
        .unwrap();
    let cost = checker.flow.work - start;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + spare;
        assert_eq!(
            checker
                .mutation_place(&program, &reports, id, Span::default())
                .is_ok(),
            spare == 0
        );
    }
    checker.flow = crate::flow::Flow::default();
    checker.place_borrows.get_mut(&id).unwrap().place.fields =
        vec![0; crate::list::MAX_WRITE_PATH + 1];
    assert!(
        checker
            .mutation_place(&program, &reports, id, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
}
