use super::super::{limits::snapshot, tests::checked};
use super::*;

#[test]
pub(crate) fn write_qualification_preserves_rhs_storage_matcher_sites_and_owners() {
    let (mut checker, reports) =
        checked("x:=1;x=2;x=3;f<int32>:(flag<boolean>){n:=7;|flag|n=8;->n}");
    let writes = checker
        .operations
        .iter()
        .filter_map(|(&id, op)| {
            (op.kind == OperationKind::Write).then_some((id, op.owner, op.local, op.input))
        })
        .collect::<Vec<_>>();
    assert_eq!(writes.len(), 3);
    assert_ne!(writes[0].3, writes[1].3);
    assert_ne!(writes[2].1, 0);
    assert_ne!(
        checker.sites[&checker.points[writes[2].0].site.unwrap()].point,
        Some(writes[2].0)
    );
    for (point, owner, local, input) in writes {
        assert_eq!(
            checker
                .write_effect(&reports, point, owner, Span::default())
                .unwrap(),
            Some(Binding {
                local,
                storage: local,
                input
            })
        );
        assert!(
            checker
                .binding_effect(&reports, point, owner, Span::default())
                .unwrap()
                .is_none()
        );
        assert!(!reports.initializers.contains_key(&local));
    }
}

#[test]
pub(crate) fn write_qualification_requires_observation_and_preserves_unknown_rhs() {
    let (mut checker, mut reports) = checked("x:=1;x=2;d:@\"debug\";x=d.panic(\"stop\");x=3");
    let writes = checker
        .operations
        .iter()
        .filter_map(|(&id, op)| (op.kind == OperationKind::Write).then_some(id))
        .collect::<Vec<_>>();
    assert_eq!(writes.len(), 3);
    for (index, point) in writes.iter().enumerate() {
        assert_eq!(
            checker
                .write_effect(&reports, *point, 0, Span::default())
                .unwrap()
                .is_some(),
            index == 0
        );
    }
    let point = writes[0];
    let op = checker.operations.get_mut(&point).unwrap();
    op.input = None;
    op.edges.drain(..2);
    let (_, Effect::Storage { input, .. }) = reports.effects.get_mut(&point).unwrap() else {
        panic!()
    };
    *input = None;
    assert!(
        checker
            .write_effect(&reports, point, 0, Span::default())
            .unwrap()
            .unwrap()
            .input
            .is_none()
    );
    reports.effects.remove(&point);
    assert!(
        checker
            .write_effect(&reports, point, 0, Span::default())
            .unwrap()
            .is_none()
    );
}

#[test]
pub(crate) fn write_qualification_rejects_stale_statement_and_permission_metadata() {
    for fault in 0..8 {
        let (mut checker, mut reports) = checked("x:=1;x=2");
        let (&point, op) = checker.operations.last_key_value().unwrap();
        let (local, input) = (op.local, op.input.unwrap());
        match fault {
            0 => checker.points[point].complete = false,
            1 => checker.points[input].parent = None,
            2 => checker.operations.get_mut(&point).unwrap().edges.swap(0, 1),
            3 => {
                checker.proofs.mutable.remove(&local);
                checker.proofs.fields.insert(local);
            }
            4 => {
                checker.proofs.bindings.remove(&local);
            }
            5 => reports.effects.get_mut(&point).unwrap().0 += 1,
            6 => checker.points[input].owner += 1,
            7 => {
                reports.index.operations.remove(&point);
            }
            _ => unreachable!(),
        }
        let before = snapshot(&checker, &reports);
        let error = checker
            .write_effect(&reports, point, 0, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity mismatch"), "fault {fault}");
        assert_eq!(before, snapshot(&checker, &reports));
    }
}

#[test]
pub(crate) fn write_qualification_keeps_alias_roots_and_rejects_changed_permissions() {
    for fault in 0..4 {
        let (mut checker, reports) =
            checked("flag:=false;row:'out{|flag|{'out->n:=1;n=2};|!flag|{'out->n:=3;n=4}}");
        let (&point, op) = checker
            .operations
            .iter()
            .rev()
            .find(|(_, op)| op.kind == OperationKind::Write)
            .unwrap();
        let (local, storage) = (op.local, op.storage);
        assert_ne!(local, storage);
        match fault {
            0 => {}
            1 => checker.proofs.aliases.get_mut(&local).unwrap().mutable = false,
            2 => checker.proofs.aliases.get_mut(&storage).unwrap().root = local,
            3 => checker.proofs.aliases.get_mut(&storage).unwrap().target = usize::MAX,
            _ => unreachable!(),
        }
        let before = snapshot(&checker, &reports);
        let result = checker.write_effect(&reports, point, 0, Span::default());
        if fault == 0 {
            assert_eq!(result.unwrap().unwrap().storage, storage);
        } else {
            assert!(result.unwrap_err().message.contains("identity mismatch"));
        }
        assert_eq!(before, snapshot(&checker, &reports));
    }
}

#[test]
pub(crate) fn write_qualification_bounds_exact_work_without_publishing_partial_reports() {
    let (mut checker, reports) = checked("x:=1;x=2;x=3");
    let start = checker.flow.work;
    checker.validate_writes(&reports, Span::default()).unwrap();
    let cost = checker.flow.work - start;
    let before = snapshot(&checker, &reports);
    for extra in [1, 0] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        assert_eq!(
            checker.validate_writes(&reports, Span::default()).is_ok(),
            extra == 0
        );
        assert_eq!(before, snapshot(&checker, &reports));
    }
}
