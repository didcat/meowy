use super::{
    tests::{expr, prepare},
    *,
};

#[test]
pub(crate) fn coercion_record_capture_preserves_original_counts_before_primary_conversion() {
    let reference = hir::Type::Reference(Box::new(hir::Type::Int {
        bits: 32,
        signed: true,
    }));
    let optional = hir::Type::union(vec![reference.clone(), hir::Type::Null]);
    let mut checker = prepare("n:1;p:&n;a:{->p;->tag:true};b:{->p;->tag:true;->other:1};c:=b");
    for (name, count) in [("a", 1), ("b", 2), ("c", 2)] {
        for target in [&reference, &optional] {
            let (id, value) = checker.expr_point(&expr(name), Some(target)).unwrap();
            let op = &checker.coercions[&id];
            assert_eq!(op.fields, Some(count));
            assert!(op.primary);
            assert_eq!(value.ty, *target);
            assert_eq!(
                op.kind,
                if *target == reference {
                    Kind::Forward
                } else {
                    Kind::Convert
                }
            );
        }
    }
    let (id, _) = checker.expr_point(&expr("p"), Some(&optional)).unwrap();
    assert_eq!(checker.coercions[&id].fields, None);
    assert_eq!(
        Coercion::primary_fields(&hir::Type::Record {
            primary: Box::new(hir::Type::Never),
            fields: Vec::new(),
        }),
        None
    );
}

#[test]
pub(crate) fn coercion_record_capture_rejects_changed_replay_and_invalid_count_shapes() {
    let mut checker = prepare("n:1;p:&n;r:{->p;->tag:true}");
    let target = hir::Type::Reference(Box::new(hir::Type::Int {
        bits: 32,
        signed: true,
    }));
    let (id, _) = checker.expr_point(&expr("r"), Some(&target)).unwrap();
    let op = checker.coercions[&id].clone();
    let edges = checker.edge_counts();
    checker
        .coercion_stages(
            (id, op.input),
            op.kind,
            op.primary,
            op.source,
            op.fields,
            op.span,
        )
        .unwrap();
    for fields in [
        None,
        Some(0),
        Some(2),
        Some(super::super::bodies::layout::MAX_SLOTS),
        Some(usize::MAX),
    ] {
        assert!(
            checker
                .coercion_stages(
                    (id, op.input),
                    op.kind,
                    op.primary,
                    op.source,
                    fields,
                    op.span
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(checker.coercions[&id], op);
        assert_eq!(checker.edge_counts(), edges);
    }
    for source in [
        None,
        Some(Shape::Never),
        Some(Shape::Scalar(super::super::ScalarKind::Bool)),
    ] {
        assert!(!Coercion::valid_fields(source, Some(1)));
    }
}

#[test]
pub(crate) fn coercion_record_capture_bounds_exact_work_and_atomic_publication() {
    let mut checker = prepare("n:1;p:&n;r:{->p;->tag:true}");
    let target = hir::Type::Reference(Box::new(hir::Type::Int {
        bits: 32,
        signed: true,
    }));
    let (id, _) = checker.expr_point(&expr("r"), Some(&target)).unwrap();
    let op = checker.coercions.remove(&id).unwrap();
    checker.coercion_edges -= op.edges.len();
    let before = format!("{:?}{:?}", checker.coercions, checker.edge_counts());
    let start = checker.flow.work;
    checker
        .coercion_stages(
            (id, op.input),
            op.kind,
            op.primary,
            op.source,
            op.fields,
            op.span,
        )
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(checker.coercions.remove(&id).unwrap(), op);
    checker.coercion_edges -= op.edges.len();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .coercion_stages(
                (id, op.input),
                op.kind,
                op.primary,
                op.source,
                op.fields,
                op.span
            )
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(
        format!("{:?}{:?}", checker.coercions, checker.edge_counts()),
        before
    );
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .coercion_stages(
            (id, op.input),
            op.kind,
            op.primary,
            op.source,
            op.fields,
            op.span,
        )
        .unwrap();
    assert_eq!(checker.coercions[&id], op);
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
}
