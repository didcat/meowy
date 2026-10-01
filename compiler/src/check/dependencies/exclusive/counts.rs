use super::{tests::check, *};

#[test]
pub(crate) fn exclusive_counts_preserve_prefix_and_indexed_field_bounds_after_transfer() {
    let source = "r:{->a:0;->inner:{->rows:=[{->xs:=[1];->z:0}]}};p:&!(r.inner.rows[1].xs[1])";
    crate::compile(source).unwrap();
    let (mut checker, body) = check(source);
    let (&id, op) = checker.exclusives.first_key_value().unwrap();
    assert_eq!(op.place.fields, [1, 0]);
    assert_eq!(op.counts, [2, 1, 2]);
    assert_eq!(op.steps[1], PathStep::Field(0));
    let before = op.clone();
    let program = hir::Program {
        body,
        functions: Vec::new(),
        locals: std::mem::take(&mut checker.locals),
    };
    checker
        .entry_reports(&program, crate::ast::Span::default())
        .unwrap();
    assert!(checker.locals.is_empty());
    assert_eq!(checker.exclusives[&id], before);
    let (checker, _) = check("f:(){r:{->xs:=[1];p:&!(xs[1]);*p=2}}");
    let op = checker.exclusives.values().next().unwrap();
    assert!(op.counts.is_empty());
    assert_eq!(op.storage, checker.proofs.aliases[&op.place.root].root);
}

#[test]
pub(crate) fn exclusive_counts_share_capture_work_and_reject_conflicting_publication() {
    let (mut checker, body) = check("r:{->rows:=[{->xs:=[1]}]};p:&!(r.rows[1].xs[1])");
    let hir::Stmt::Bind { value, .. } = &body.stmts[1] else {
        panic!()
    };
    let (&id, op) = checker.exclusives.first_key_value().unwrap();
    let op = op.clone();
    let plan = Plan {
        steps: op.steps.clone(),
        access: op.access.clone(),
    };
    checker.exclusives.clear();
    checker.exclusive_edges = 0;
    let before = checker.flow.work;
    checker
        .exclusive_operation(id, value, plan.clone())
        .unwrap();
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.exclusives.clear();
        checker.exclusive_edges = 0;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.exclusive_operation(id, value, plan.clone());
        assert_eq!(result.is_ok(), spare == 0);
        if result.is_ok() {
            assert_eq!(checker.exclusives[&id], op);
        } else {
            assert!(checker.exclusives.is_empty());
            assert_eq!(checker.exclusive_edges, 0);
        }
    }
    checker.flow = crate::flow::Flow::default();
    checker
        .exclusive_operation(id, value, plan.clone())
        .unwrap();
    checker.exclusives.get_mut(&id).unwrap().counts[0] += 1;
    let before = checker.exclusives.clone();
    let edges = checker.exclusive_edges;
    assert!(
        checker
            .exclusive_operation(id, value, plan)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.exclusives, before);
    assert_eq!(checker.exclusive_edges, edges);
}
