use super::{tests::checked, *};

pub(super) fn snapshot(checker: &Checker, reports: &Reports) -> String {
    format!(
        "{reports:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
        checker.operations,
        checker.points,
        checker.sites,
        checker.bodies,
        checker.proofs.bindings,
        checker
            .proofs
            .aliases
            .iter()
            .map(|(id, alias)| (id, alias.root, alias.target, alias.emission, alias.span))
            .collect::<Vec<_>>(),
        checker.local_reads,
        checker.edge_counts()
    )
}

#[test]
pub(crate) fn binding_qualification_rejects_observed_and_checked_identity_conflicts() {
    for fault in 0..35 {
        let (mut checker, mut reports) = checked("x:1;r:{->n:2}");
        let id = *checker.operations.first_key_value().unwrap().0;
        let child = checker.operations[&id].input.unwrap();
        let block = checker.points[id].block.unwrap();
        let site = checker.points[id].site.unwrap();
        let (
            owner,
            Effect::Storage {
                kind,
                local,
                storage,
                input,
                control,
            },
        ) = reports.effects.get_mut(&id).unwrap()
        else {
            panic!()
        };
        match fault {
            0 => *owner = 9,
            1 => checker.operations.get_mut(&id).unwrap().owner = 9,
            2 => *kind = OperationKind::Write,
            3 => *local += 1,
            4 => *storage += 1,
            5 => *input = Some(id),
            6 => *control = !*control,
            7 => {
                checker.operations.remove(&id);
            }
            8 => checker.points[id].complete = false,
            9 => checker.points[id].kind = PointKind::Expr,
            10 => checker.points[id].owner = 9,
            11 => checker.points[id].span.end += 1,
            12 => checker.points[id].block = None,
            13 => checker.points[id].site = None,
            14 => {
                reports.index.operations.insert(id, 9);
            }
            15 => {
                checker.proofs.bindings.remove(local);
            }
            16 => checker.bodies.get_mut(&block).unwrap().owner = 9,
            17 => checker.points[child].parent = None,
            18 => checker.points[child].owner = 9,
            19 => checker.points[child].block = None,
            20 => checker.points[child].site = None,
            21 => checker.points[child].kind = PointKind::Stmt,
            22 => checker.points[child].complete = false,
            23 => checker.points[child].span.end = checker.points[id].span.end + 1,
            24 => checker.operations.get_mut(&id).unwrap().edges[1].route = Route::Returned,
            25 => checker.operations.get_mut(&id).unwrap().edges.swap(0, 1),
            26 => checker.sites.get_mut(&site).unwrap().complete = false,
            27 => checker.sites.get_mut(&site).unwrap().owner = 9,
            28 => checker.sites.get_mut(&site).unwrap().point = Some(child),
            29 => checker.sites.get_mut(&site).unwrap().span.end += 1,
            30 => {
                *local = reports.locals;
                checker.operations.get_mut(&id).unwrap().local = *local;
            }
            31 => {
                *storage = reports.locals;
                checker.operations.get_mut(&id).unwrap().storage = *storage;
            }
            32 => {
                checker.bodies.remove(&block);
            }
            33 => {
                checker.sites.remove(&site);
            }
            34 => {
                let mut alias = checker.proofs.aliases.values().next().unwrap().clone();
                alias.root = *local + 1;
                checker.proofs.aliases.insert(*local, alias);
            }
            _ => unreachable!(),
        }
        let before = snapshot(&checker, &reports);
        let error = checker
            .binding_effect(&reports, id, 0, Span::default())
            .unwrap_err();
        assert!(
            error.message.contains("binding identity mismatch"),
            "fault {fault}"
        );
        assert_eq!(snapshot(&checker, &reports), before, "fault {fault}");
    }
}

#[test]
pub(crate) fn binding_qualification_preserves_missing_write_and_unknown_input_boundaries() {
    for state in 0..6 {
        let (mut checker, mut reports) = checked("x:1");
        let id = *checker.operations.first_key_value().unwrap().0;
        let op = checker.operations.get_mut(&id).unwrap();
        let local = op.local;
        let storage = op.storage;
        let (_, Effect::Storage { kind, input, .. }) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match state {
            0 => {
                reports.effects.remove(&id);
            }
            1 => {
                *kind = OperationKind::Write;
                op.kind = *kind;
            }
            2 => {
                *input = None;
                op.input = None;
                op.edges.drain(..2);
            }
            3 => *input = None,
            4 => {
                *input = None;
                op.input = None;
            }
            5 => {
                *input = None;
                op.input = None;
                op.edges.clear();
            }
            _ => unreachable!(),
        }
        let before = snapshot(&checker, &reports);
        let result = checker.binding_effect(&reports, id, 0, Span::default());
        if state < 2 {
            assert_eq!(result.unwrap(), None);
        } else if state == 2 {
            assert_eq!(
                result.unwrap(),
                Some(Binding {
                    local,
                    storage,
                    input: None
                })
            );
        } else {
            assert!(
                result
                    .unwrap_err()
                    .message
                    .contains("binding identity mismatch")
            );
        }
        assert_eq!(snapshot(&checker, &reports), before, "state {state}");
    }
}

#[test]
pub(crate) fn binding_qualification_rejects_cyclic_matcher_parent_chains() {
    let (mut checker, reports) = checked("f<int32>:(flag<boolean>){|flag|x:1;n:2;->n}");
    let id = *checker
        .operations
        .iter()
        .find(|(id, _)| checker.sites[&checker.points[**id].site.unwrap()].point != Some(**id))
        .unwrap()
        .0;
    let site = checker.points[id].site.unwrap();
    let owner = checker.operations[&id].owner;
    assert_ne!(checker.sites[&site].point, Some(id));
    checker.points[id].parent = Some(id);
    let before = snapshot(&checker, &reports);
    let error = checker
        .binding_effect(&reports, id, owner, Span::default())
        .unwrap_err();
    assert!(error.message.contains("binding identity mismatch"));
    assert_eq!(snapshot(&checker, &reports), before);
}

#[test]
pub(crate) fn binding_qualification_bounds_work_and_keeps_late_failures_atomic() {
    for state in 0..3 {
        let (mut checker, mut reports) = checked("x:1");
        let id = *checker.operations.first_key_value().unwrap().0;
        if state == 1 {
            reports.effects.remove(&id);
        } else if state == 2 {
            let (_, Effect::Storage { kind, .. }) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            *kind = OperationKind::Write;
            checker.operations.get_mut(&id).unwrap().kind = *kind;
        }
        let before = snapshot(&checker, &reports);
        let start = checker.flow.work;
        let expected = checker
            .binding_effect(&reports, id, 0, Span::default())
            .unwrap();
        let work = checker.flow.work - start;
        for spare in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
            let result = checker.binding_effect(&reports, id, 0, Span::default());
            if spare == 0 {
                assert_eq!(result.unwrap(), expected);
            } else {
                assert!(
                    result
                        .unwrap_err()
                        .message
                        .contains("binding budget exhausted")
                );
            }
            assert_eq!(snapshot(&checker, &reports), before);
        }
    }
    let (mut checker, reports) = checked("a:1;b:2;c:3");
    let before = snapshot(&checker, &reports);
    let start = checker.flow.work;
    checker
        .validate_bindings(&reports, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.validate_bindings(&reports, Span::default());
        if spare == 0 {
            result.unwrap();
        } else {
            assert!(
                result
                    .unwrap_err()
                    .message
                    .contains("binding budget exhausted")
            );
        }
        assert_eq!(snapshot(&checker, &reports), before);
    }
    let (mut checker, reports) = checked("a:1;b:2;c:3");
    checker
        .operations
        .last_entry()
        .unwrap()
        .get_mut()
        .edges
        .pop();
    let before = snapshot(&checker, &reports);
    let error = checker
        .validate_bindings(&reports, Span::default())
        .unwrap_err();
    assert!(error.message.contains("binding identity mismatch"));
    assert_eq!(snapshot(&checker, &reports), before);
}
