use super::{tests::checked, *};
use crate::check::dependencies::OperationKind;

pub(super) fn snapshot(checker: &Checker, program: &hir::Program, reports: &Reports) -> String {
    format!(
        "{program:?}{reports:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
        checker.operations,
        checker.points,
        checker.sites,
        checker.bodies,
        checker.proofs.bindings,
        checker.proofs.receivers,
        checker.proofs.temporaries,
        checker
            .proofs
            .aliases
            .iter()
            .map(|(id, alias)| (id, alias.root, alias.target, alias.emission, alias.span))
            .collect::<Vec<_>>(),
        checker.edge_counts()
    )
}

#[test]
pub(crate) fn initializer_index_rejects_duplicate_local_statements_atomically() {
    let (mut checker, program, mut reports) = checked("a:1;b:2");
    let (&first, prior) = reports.initializers.first_key_value().unwrap();
    let last = reports.initializers.last_key_value().unwrap().1.statement;
    assert_ne!(prior.statement, last);
    let op = checker.operations.get_mut(&last).unwrap();
    op.local = first;
    op.storage = first;
    let (_, Effect::Storage { local, storage, .. }) = reports.effects.get_mut(&last).unwrap()
    else {
        panic!()
    };
    *local = first;
    *storage = first;
    let before = snapshot(&checker, &program, &reports);
    assert!(
        checker
            .binding_initializers(&program, &reports, Span::default())
            .unwrap_err()
            .message
            .contains("initializer-index identity mismatch")
    );
    assert_eq!(snapshot(&checker, &program, &reports), before);
}

#[test]
pub(crate) fn initializer_index_keeps_missing_inputs_without_inventing_roots() {
    let (mut checker, program, mut reports) = checked("a:1");
    let (&local, prior) = reports.initializers.first_key_value().unwrap();
    let id = prior.statement;
    let owner = prior.owner;
    let op = checker.operations.get_mut(&id).unwrap();
    op.input = None;
    op.edges.drain(..2);
    let (_, Effect::Storage { input, .. }) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    *input = None;
    let before = snapshot(&checker, &program, &reports);
    let index = checker
        .binding_initializers(&program, &reports, Span::default())
        .unwrap();
    assert_eq!(
        index[&local],
        Initializer {
            statement: id,
            owner,
            input: None
        }
    );
    assert_eq!(snapshot(&checker, &program, &reports), before);
}

#[test]
pub(crate) fn initializer_index_filters_special_local_identities_after_qualification() {
    for state in 0..7 {
        let (mut checker, mut program, mut reports) =
            checked("a:1;box:{->n:2};f<int32>:(p<int32>){->p}");
        let (&local, prior) = reports.initializers.first_key_value().unwrap();
        let id = prior.statement;
        match state {
            0 => {
                let mut alias = checker.proofs.aliases.values().next().unwrap().clone();
                alias.root = local;
                checker.proofs.aliases.insert(local, alias);
            }
            1 => program.functions[0].params.push(local),
            2 => {
                checker.proofs.receivers.insert(local);
            }
            3 => {
                checker
                    .proofs
                    .temporaries
                    .insert(local, checker.points[id].site.unwrap());
            }
            4 => {
                reports.eligible.remove(&local);
            }
            5 => {
                checker.operations.get_mut(&id).unwrap().kind = OperationKind::Write;
                let (_, Effect::Storage { kind, .. }) = reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *kind = OperationKind::Write;
            }
            6 => {
                reports.effects.remove(&id);
            }
            _ => unreachable!(),
        }
        let before = snapshot(&checker, &program, &reports);
        let index = checker
            .binding_initializers(&program, &reports, Span::default())
            .unwrap();
        assert!(!index.contains_key(&local), "state {state}");
        assert_eq!(snapshot(&checker, &program, &reports), before);
        if state == 6 {
            continue;
        }
        let (_, Effect::Storage { control, .. }) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        *control = !*control;
        let before = snapshot(&checker, &program, &reports);
        let error = checker
            .binding_initializers(&program, &reports, Span::default())
            .unwrap_err();
        assert!(
            error.message.contains("binding identity mismatch"),
            "state {state}"
        );
        assert_eq!(snapshot(&checker, &program, &reports), before);
    }
}

#[test]
pub(crate) fn initializer_index_rejects_missing_owner_entries() {
    let (mut checker, program, mut reports) = checked("a:1;f<int32>:(){b:2;->b}");
    let owner = reports
        .initializers
        .values()
        .find(|entry| entry.owner != 0)
        .unwrap()
        .owner;
    reports.entries.remove(&owner);
    let before = snapshot(&checker, &program, &reports);
    let error = checker
        .binding_initializers(&program, &reports, Span::default())
        .unwrap_err();
    assert!(error.message.contains("identity mismatch"));
    assert_eq!(snapshot(&checker, &program, &reports), before);
}

#[test]
pub(crate) fn initializer_index_shares_map_capacity_without_counting_payloads_twice() {
    let (mut checker, program, mut reports) = checked("a:({->n:1}).n;b:2;copy:b");
    let shared = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.slot_uses.len()
        + reports.eligible.len();
    assert!(!reports.slot_uses.is_empty());
    assert!(!reports.initializers.is_empty());
    reports.parts = usize::MAX;
    let exact = shared + reports.initializers.len();
    let before = snapshot(&checker, &program, &reports);
    for limit in [shared - 1, exact - 1, exact] {
        let result =
            checker.binding_initializers_limited(&program, &reports, Span::default(), limit);
        if limit == exact {
            assert_eq!(result.unwrap(), reports.initializers);
        } else {
            assert!(
                result
                    .unwrap_err()
                    .message
                    .contains("initializer-index budget exhausted")
            );
        }
        assert_eq!(snapshot(&checker, &program, &reports), before);
    }
}

#[test]
pub(crate) fn initializer_index_bounds_exact_work_and_keeps_late_failures_atomic() {
    let (mut checker, program, reports) = checked("a:1;b:{->n:2};c:3;f<int32>:(p<int32>){->p}");
    let before = snapshot(&checker, &program, &reports);
    let start = checker.flow.work;
    let expected = checker
        .binding_initializers(&program, &reports, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    for spare in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.binding_initializers(&program, &reports, Span::default());
        if spare == 0 {
            assert_eq!(result.unwrap(), expected);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget exhausted"));
        }
        assert_eq!(snapshot(&checker, &program, &reports), before);
    }
    checker.flow = crate::flow::Flow::new();
    checker
        .operations
        .last_entry()
        .unwrap()
        .get_mut()
        .edges
        .pop();
    let before = snapshot(&checker, &program, &reports);
    let error = checker
        .binding_initializers(&program, &reports, Span::default())
        .unwrap_err();
    assert!(error.message.contains("binding identity mismatch"));
    assert_eq!(snapshot(&checker, &program, &reports), before);
}
