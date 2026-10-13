use super::{super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn shared_initializer_index_keeps_original_roots_owners_and_special_cell_exclusions() {
    let source = "n:1;p:&n;r:{->p;->tag:true};copy:((r));out:r.{alias:$;box:{->named:$;copy:named}};mutable:={->p;->tag:true};f: (p<&int32>,flag<boolean>){|flag|r:{->p;->tag:true}}";
    let (mut checker, mut program, reports) = checked(source);
    assert!(reports.shared_initializers.len() >= 5);
    assert!(
        reports
            .shared_initializers
            .values()
            .any(|row| row.init.owner != 0)
    );
    for (&local, row) in &reports.shared_initializers {
        assert!(!reports.eligible.contains(&local));
        assert!(!reports.initializers.contains_key(&local));
        assert!(!checker.proofs.variable(local));
        assert!(!checker.proofs.aliases.contains_key(&local));
        assert!(!checker.proofs.receivers.contains(&local));
        assert!(!checker.proofs.temporaries.contains_key(&local));
        let op = &checker.operations[&row.init.statement];
        assert_eq!(row.init.input, op.input);
        assert_eq!(row.init.owner, op.owner);
        assert_eq!(Some(row.record), op.shared);
        assert_eq!(op.local, local);
        assert_eq!(op.storage, local);
    }
    program.functions.reverse();
    let count = reports.shared_initializers.len();
    assert_eq!(
        checker
            .shared_initializers(&program, &reports, Span::default(), count, count)
            .unwrap(),
        (reports.shared_initializers.clone(), 0)
    );
}

#[test]
pub(crate) fn shared_initializer_index_rejects_stale_types_and_descriptors_atomically() {
    for fault in 0..6 {
        let (mut checker, mut program, reports) = checked("n:1;p:&n;r:{->p;->tag:true};copy:r");
        let (&local, row) = reports.shared_initializers.last_key_value().unwrap();
        let hir::Type::Record { primary, fields } = &mut program.locals[local] else {
            panic!()
        };
        match fault {
            0 => **primary = hir::Type::Reference(Box::new(hir::Type::Bool)),
            1 => fields[0].mutable = true,
            2 => fields[0].ty = hir::Type::Reference(Box::new(hir::Type::Bool)),
            3 => fields.clear(),
            4 => {
                checker
                    .operations
                    .get_mut(&row.init.statement)
                    .unwrap()
                    .shared = None
            }
            5 => {
                checker
                    .operations
                    .get_mut(&row.init.statement)
                    .unwrap()
                    .shared
                    .as_mut()
                    .unwrap()
                    .primary = ScalarKind::Float { bits: 0 }
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .shared_initializers(&program, &reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_initializer_index_bounds_rows_payload_and_exact_work() {
    let (mut checker, program, reports) = checked("n:1;p:&n;r:{->p;->tag:true};copy:r");
    let count = reports.shared_initializers.len();
    assert_eq!(count, 2);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let expected = checker
        .shared_initializers(&program, &reports, Span::default(), count, count)
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(expected, (reports.shared_initializers.clone(), 0));
    for (limit, parts) in [(count - 1, count), (count, count - 1)] {
        assert!(
            checker
                .shared_initializers(&program, &reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.shared_initializers(&program, &reports, Span::default(), count, count);
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn shared_initializer_index_rejects_duplicate_binds_and_skips_missing_observations() {
    let (mut checker, program, mut reports) = checked("n:1;p:&n;r:{->p;->tag:true};copy:r");
    let (&first, _) = reports.shared_initializers.first_key_value().unwrap();
    let (&last, row) = reports.shared_initializers.last_key_value().unwrap();
    let statement = row.init.statement;
    let effect = reports.effects.remove(&statement).unwrap();
    let mut expected = reports.shared_initializers.clone();
    expected.remove(&last);
    assert_eq!(
        checker
            .shared_initializers(&program, &reports, Span::default(), 1, 1)
            .unwrap(),
        (expected, 0)
    );
    reports.effects.insert(statement, effect);
    let op = checker.operations.get_mut(&statement).unwrap();
    op.local = first;
    op.storage = first;
    let (_, Effect::Storage { local, storage, .. }) = reports.effects.get_mut(&statement).unwrap()
    else {
        panic!()
    };
    *local = first;
    *storage = first;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .shared_initializers(&program, &reports, Span::default(), 2, 2)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
