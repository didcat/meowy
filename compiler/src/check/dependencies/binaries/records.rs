use super::{tests::check, *};

#[test]
pub(crate) fn binary_record_capture_keeps_counts_only_for_owned_shared_primary_operands() {
    for (tail, expected) in [
        ("a==p", [Some(1), None]),
        ("p!=b", [None, Some(2)]),
        ("a==a", [None; 2]),
        ("p==p", [None; 2]),
        ("out:b.{p!=(($))}", [None; 2]),
    ] {
        let source = format!("n:1;p:&n;a:{{->p;->tag:true}};b:{{->p;->tag:true;->other:3}};{tail}");
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let op = checker.binaries.values().next().unwrap();
        assert_eq!(op.fields, expected, "{source}");
        assert!(Binary::valid_fields(op.types, op.plan.primary, op.fields));
    }
    let (checker, _) = check("a:{->1;->tag:true};a+2");
    let op = checker.binaries.values().next().unwrap();
    assert_eq!(op.fields, [None; 2]);
    assert!(!Binary::valid_fields(
        op.types,
        op.plan.primary,
        [Some(1), None]
    ));
}

#[test]
pub(crate) fn binary_record_capture_rejects_changed_original_counts_and_referents_atomically() {
    for fault in 0..3 {
        let (mut checker, body) = check("n:1;p:&n;r:{->p;->tag:true};r==p");
        let hir::Stmt::Expr(value) = body.stmts.last().unwrap() else {
            panic!()
        };
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        let op = op.clone();
        let mut value = value.clone();
        let hir::ExprKind::Binary { left, .. } = &mut value.kind else {
            panic!()
        };
        let hir::ExprKind::Primary(input) = &mut left.kind else {
            panic!()
        };
        let hir::Type::Record { primary, fields } = &mut input.ty else {
            panic!()
        };
        match fault {
            0 => fields.clear(),
            1 => **primary = hir::Type::Reference(Box::new(hir::Type::Bool)),
            2 => fields.resize(super::super::bodies::layout::MAX_SLOTS, fields[0].clone()),
            _ => unreachable!(),
        }
        let before = format!(
            "{:?}{:?}{:?}",
            checker.binaries,
            checker.sequences,
            checker.edge_counts()
        );
        assert!(
            checker
                .binary_operation(id, op.inputs, op.plan, &value, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(
            format!(
                "{:?}{:?}{:?}",
                checker.binaries,
                checker.sequences,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn binary_record_capture_bounds_exact_work_and_publishes_both_ledgers_atomically() {
    let (mut checker, body) = check("n:1;p:&n;r:{->p;->tag:true};p==r");
    let hir::Stmt::Expr(value) = body.stmts.last().unwrap() else {
        panic!()
    };
    let id = *checker.binaries.first_key_value().unwrap().0;
    let op = checker.binaries.remove(&id).unwrap();
    let key = SequenceSource::Expr(id);
    let seq = checker.sequences.remove(&key).unwrap();
    checker.binary_edges -= op.edges.len();
    checker.sequence_edges -= seq.edges.len();
    let before = format!(
        "{:?}{:?}{:?}",
        checker.binaries,
        checker.sequences,
        checker.edge_counts()
    );
    let start = checker.flow.work;
    checker
        .binary_operation(id, op.inputs, op.plan, value, op.span)
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(checker.binaries.remove(&id).unwrap(), op);
    assert_eq!(checker.sequences.remove(&key).unwrap(), seq);
    checker.binary_edges -= op.edges.len();
    checker.sequence_edges -= seq.edges.len();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .binary_operation(id, op.inputs, op.plan, value, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(
        format!(
            "{:?}{:?}{:?}",
            checker.binaries,
            checker.sequences,
            checker.edge_counts()
        ),
        before
    );
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .binary_operation(id, op.inputs, op.plan, value, op.span)
        .unwrap();
    assert_eq!(checker.binaries[&id], op);
    assert_eq!(checker.sequences[&key], seq);
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
}
