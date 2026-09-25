use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn exclusive_paths_reserve_each_container_before_indices_and_acquisition() {
    let source = "r:={->rows:=[{->xs:=[1]}]};n:=0;p:&!(r.rows[{n=1;->1}].xs[1]);*p=2";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let (&id, op) = checker.exclusives.first_key_value().unwrap();
    assert_eq!(op.place.fields, [0]);
    assert_eq!(op.storage, op.place.root);
    assert_eq!(op.steps[1], PathStep::Field(0));
    for (step, input) in op.steps.iter().enumerate() {
        if let PathStep::Index { point, .. } = input {
            let reserve = Port::Reserve { point: id, step };
            assert!(op.edges.contains(&Edge::new(
                Port::Address { point: id, step },
                reserve,
                Route::Next
            )));
            assert!(
                op.edges
                    .contains(&Edge::new(reserve, Port::Entry(*point), Route::Next))
            );
            assert!(op.edges.contains(&Edge::new(
                Port::Normal(*point),
                Port::Address {
                    point: id,
                    step: step + 1
                },
                Route::Checked
            )));
        }
    }
    assert!(op.edges.contains(&Edge::new(
        Port::Address { point: id, step: 3 },
        Port::Operation(id),
        Route::Next
    )));
    assert!(op.edges.contains(&Edge::new(
        Port::Operation(id),
        Port::Normal(id),
        Route::Next
    )));
    assert!(
        !op.edges
            .iter()
            .any(|edge| edge.from == Port::Entry(id) && edge.to == Port::Normal(id))
    );
}

#[test]
pub(crate) fn exclusive_paths_keep_stopped_indices_aliases_and_function_owners() {
    for source in [
        "xs:=[[1]];'out{p:&!(xs[{'out.leave()}][1])}",
        "xs:=[[1]];'out{p:&!(xs[1][{'out.leave()}])}",
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, op) = checker.exclusives.first_key_value().unwrap();
        assert!(
            !op.edges
                .iter()
                .any(|edge| edge.to == Port::Operation(id) || edge.to == Port::Normal(id))
        );
        assert_eq!(
            op.edges
                .iter()
                .filter(|edge| edge.route == Route::Checked)
                .count(),
            1
        );
    }
    let (checker, _) = check("f:(){r:{->xs:=[1];p:&!(xs[1]);*p=2}}");
    let op = checker.exclusives.values().next().unwrap();
    assert_ne!(op.owner, 0);
    assert_eq!(op.storage, checker.proofs.aliases[&op.place.root].root);
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;xs:=[1];|flag|{p:&!(xs[1])}").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.exclusives.values().next().unwrap().control);
}

#[test]
pub(crate) fn exclusive_paths_publish_atomically_with_valid_identity_and_shared_budgets() {
    let (mut checker, block) = check("xs:=[[1]];p:&!(xs[1][1])");
    let hir::Stmt::Bind { value, .. } = &block.stmts[1] else {
        panic!()
    };
    let (&id, op) = checker.exclusives.first_key_value().unwrap();
    let steps = op.steps.clone();
    let count = checker.exclusive_edges;
    checker
        .exclusive_operation(id, value, steps.clone())
        .unwrap();
    assert_eq!(checker.exclusive_edges, count);
    checker.exclusives.clear();
    checker.exclusive_edges = 0;
    let mut repeated = steps.clone();
    repeated[1] = steps[0].clone();
    assert!(
        checker
            .exclusive_operation(id, value, repeated)
            .unwrap_err()
            .message
            .contains("identity")
    );
    let PathStep::Index { point, .. } = steps[0] else {
        panic!()
    };
    checker.points[point].parent = None;
    assert!(
        checker
            .exclusive_operation(id, value, steps.clone())
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[point].parent = Some(id);
    checker.element_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .exclusive_operation(id, value, steps)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.exclusives.is_empty());
    assert_eq!(checker.exclusive_edges, 0);
}
