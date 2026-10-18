use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;

pub(super) const GUARDED: &str = "f:(flag<boolean>)'out{|flag|{'out->r:{->n:=1;->tail:false};p:&!(r.n)};|!flag|{'out->r:{->n:=2;->tail:true};q:&!(r.n)}}";

#[test]
pub(crate) fn call_alias_storage_qualifies_scalars_fields_and_guarded_canonical_roots() {
    for source in [
        "r:{->n:=1;p:&!n}",
        "r:{->inner:{->n:=1;->tail:false};p:&!(inner.n)}",
        GUARDED,
    ] {
        crate::compile(source).unwrap();
        let (mut checker, program) = checked(source);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let blocks = checker
            .program_blocks(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        let ids = checker.place_borrows.keys().copied().collect::<Vec<_>>();
        for id in ids {
            assert!(
                checker
                    .mutation_slot(&program, &reports, &blocks, id, Span::default())
                    .unwrap()
                    .is_some()
            );
        }
    }
}

#[test]
pub(crate) fn call_alias_storage_rejects_stale_roots_targets_views_and_full_backings() {
    for fault in 0..12 {
        let (mut checker, mut program) = checked(GUARDED);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let (&id, op) = checker.place_borrows.last_key_value().unwrap();
        let view = op.place.root;
        let alias = checker.proofs.aliases.get_mut(&view).unwrap();
        let root = alias.root;
        assert_ne!(root, view);
        match fault {
            0 => alias.target = usize::MAX,
            1 => alias.root = view,
            2 => alias.field = "missing".into(),
            3 => alias.mutable = !alias.mutable,
            4 => alias.backing = None,
            5 => alias.exclusive = None,
            6 => alias.emission = usize::MAX,
            7 => checker.proofs.aliases.get_mut(&root).unwrap().target = usize::MAX,
            8 => program.locals[root] = Type::Bool,
            9 => program.locals[view] = Type::Bool,
            10 => {
                checker.proofs.bindings.remove(&root);
            }
            11 => {
                let Type::Record { fields, .. } = &mut program.functions[0].body.ty else {
                    panic!()
                };
                let Type::Record { fields, .. } = &mut fields[0].ty else {
                    panic!()
                };
                fields[1].mutable = !fields[1].mutable;
            }
            _ => unreachable!(),
        }
        let blocks = checker
            .program_blocks(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .mutation_slot(&program, &reports, &blocks, id, Span::default())
            .unwrap_err();
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn call_alias_storage_bounds_exact_work_and_keeps_discarded_storage_unknown() {
    let (mut checker, program) = checked("r:{->n:=1;p:&!n}");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let blocks = checker
        .program_blocks(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (&id, op) = checker.place_borrows.first_key_value().unwrap();
    let local = op.place.root;
    let start = checker.flow.work;
    checker
        .mutation_slot(&program, &reports, &blocks, id, Span::default())
        .unwrap();
    let cost = checker.flow.work - start;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + spare;
        assert_eq!(
            checker
                .mutation_slot(&program, &reports, &blocks, id, Span::default())
                .is_ok(),
            spare == 0
        );
    }
    checker.flow = crate::flow::Flow::default();
    checker.proofs.aliases.get_mut(&local).unwrap().backing = Some(Backing::Discarded);
    assert_eq!(
        checker
            .mutation_slot(&program, &reports, &blocks, id, Span::default())
            .unwrap(),
        None
    );
}
