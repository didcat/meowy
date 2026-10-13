use super::{tests::check, *};

#[test]
pub(crate) fn shared_binding_capture_retains_original_primary_and_field_count() {
    let source = "n<uint64>:1;p:&n;r:{->p;->tag:true};copy:r;mutable:={->p;->tag:true};mutable=r;f:(){n<float32>:1.5;r:{->&n;->tag:true}}";
    crate::compile(source).unwrap();
    let mut checker = check(source);
    let mut count = 0;
    for (&id, op) in &checker.operations {
        let expected = if op.kind == Kind::Bind {
            Checker::shared_record_type(&checker.locals[op.local], &mut checker.flow, op.span)
                .unwrap()
        } else {
            None
        };
        assert_eq!(op.shared, expected);
        if let Some(shared) = op.shared {
            assert_eq!(shared.fields, 1);
            assert_eq!(
                shared.primary,
                if op.owner == 0 {
                    ScalarKind::Int {
                        bits: 64,
                        signed: false,
                    }
                } else {
                    ScalarKind::Float { bits: 32 }
                }
            );
            assert_eq!(checker.points[op.input.unwrap()].parent, Some(id));
            count += 1;
        }
    }
    assert_eq!(count, 4);
}

#[test]
pub(crate) fn shared_binding_capture_preserves_replay_and_rejects_stale_original_types() {
    for fault in 0..4 {
        let mut checker = check("n:1;p:&n;r:{->p;->tag:true}");
        let (&id, op) = checker
            .operations
            .iter()
            .find(|(_, op)| op.shared.is_some())
            .unwrap();
        let (local, input, span) = (op.local, op.input, op.span);
        let before = format!("{:?}{:?}", checker.operations, checker.edge_counts());
        checker
            .storage_operation(id, Kind::Bind, local, input, span)
            .unwrap();
        assert_eq!(
            format!("{:?}{:?}", checker.operations, checker.edge_counts()),
            before
        );
        let hir::Type::Record { primary, fields } = &mut checker.locals[local] else {
            panic!()
        };
        match fault {
            0 => **primary = hir::Type::Reference(Box::new(hir::Type::Bool)),
            1 => fields[0].mutable = true,
            2 => fields[0].ty = hir::Type::Reference(Box::new(hir::Type::Bool)),
            3 => fields.clear(),
            _ => unreachable!(),
        }
        assert!(
            checker
                .storage_operation(id, Kind::Bind, local, input, span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(
            format!("{:?}{:?}", checker.operations, checker.edge_counts()),
            before
        );
    }
}

#[test]
pub(crate) fn shared_binding_capture_bounds_exact_type_work_and_atomic_publication() {
    let mut checker = check("n:1;p:&n;r:{->p;->tag:true;->inner:{->items:[1]}}");
    let (&id, op) = checker
        .operations
        .iter()
        .find(|(_, op)| op.shared.is_some())
        .unwrap();
    let (local, input, span) = (op.local, op.input, op.span);
    let op = checker.operations.remove(&id).unwrap();
    checker.operation_edges -= op.edges.len();
    let before = format!("{:?}{:?}", checker.operations, checker.edge_counts());
    let start = checker.flow.work;
    checker
        .storage_operation(id, Kind::Bind, local, input, span)
        .unwrap();
    let work = checker.flow.work - start;
    let actual = checker.operations.remove(&id).unwrap();
    assert_eq!(actual, op);
    checker.operation_edges -= actual.edges.len();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .storage_operation(id, Kind::Bind, local, input, span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(
        format!("{:?}{:?}", checker.operations, checker.edge_counts()),
        before
    );
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .storage_operation(id, Kind::Bind, local, input, span)
        .unwrap();
    assert_eq!(checker.operations[&id], op);
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
}
