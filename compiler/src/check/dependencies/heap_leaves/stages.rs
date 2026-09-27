use super::{
    tests::{expr, setup},
    *,
};

#[test]
pub(crate) fn heap_leaf_stages_precede_expected_conversion_and_call_consumers() {
    let mut checker = setup("m:@\"memory\"");
    let ty = hir::Type::union(vec![
        hir::Type::Foundation(hir::FoundationType::Allocator),
        hir::Type::Null,
    ]);
    let (outer, _) = checker.expr_point(&expr("m.heap"), Some(&ty)).unwrap();
    let leaf = checker.coercions[&outer].input;
    assert_eq!(
        checker.heap_leaves[&leaf].edges,
        [
            Edge::new(Port::Entry(leaf), Port::Operation(leaf), Route::Next),
            Edge::new(Port::Operation(leaf), Port::Normal(leaf), Route::Next),
        ]
    );
    assert!(checker.coercions[&outer].edges.contains(&Edge::new(
        Port::Normal(leaf),
        Port::Operation(outer),
        Route::Next
    )));
    let source = "m:@\"memory\";f<m.Allocator>:(p<m.Allocator>){->p};a:f(m.heap)";
    crate::compile(source).unwrap();
    let checker = setup(source);
    assert_eq!(checker.invocations.len(), 1);
    let call = checker.invocations.values().next().unwrap();
    let input = call.args[0];
    let leaf = checker.coercions[&input].input;
    assert!(checker.heap_leaves.contains_key(&leaf));
    assert!(call.edges.contains(&Edge::new(
        Port::Normal(input),
        Port::Operation(call.point),
        Route::Next
    )));
}

#[test]
pub(crate) fn heap_leaf_stages_precede_temporary_materialization_without_extending_lifetimes() {
    let source = "m:@\"memory\";copy:*(&{->m.heap})";
    crate::compile(source).unwrap();
    let checker = setup(source);
    let (&id, temp) = checker.temporary_borrows.first_key_value().unwrap();
    let (&leaf, _) = checker.heap_leaves.first_key_value().unwrap();
    let block = checker.points[leaf].block.unwrap();
    assert_eq!(checker.bodies[&block].parent, Some(temp.input));
    assert!(
        checker.endpoints[&super::super::SequenceSource::Expr(temp.input)].contains(&Edge::new(
            Port::BlockResult(block),
            Port::Normal(temp.input),
            Route::Result
        ))
    );
    assert!(temp.edges.contains(&Edge::new(
        Port::Normal(temp.input),
        Port::Operation(id),
        Route::Next
    )));
    for (source, code) in [
        ("m:@\"memory\";p:&{->m.heap};copy:*p", "E303"),
        ("m:@\"memory\";a:=m.heap;p:&a;a=m.heap;copy:*p", "E302"),
        ("m:@\"memory\";a:m.heap;same:a==a", "E222"),
        ("m:@\"memory\";copy:*(&m.heap)", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn heap_leaf_stages_keep_owner_control_and_hint_required_boundaries() {
    let source = "m:@\"memory\";flag:false;|flag|a:m.heap;f<m.Allocator>:(){->m.heap}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert!(checker.heap_leaves.values().any(|leaf| leaf.control));
    assert!(checker.heap_leaves.values().any(|leaf| leaf.owner != 0));
    let mut checker = setup("m:@\"memory\"");
    assert_eq!(
        checker.hint(&expr("m.heap")),
        Some(hir::Type::Foundation(hir::FoundationType::Allocator))
    );
    assert!(checker.heap_leaves.is_empty());
    checker.required = true;
    checker.expr_point(&expr("m.heap"), None).unwrap();
    assert!(checker.heap_leaves.is_empty());
}

#[test]
pub(crate) fn heap_leaf_stages_validate_nominal_shapes_and_shared_budget_atomically() {
    let mut checker = setup("m:@\"memory\"");
    let (id, value) = checker.expr_point(&expr("m.heap"), None).unwrap();
    let count = checker.heap_leaf_edges;
    checker.capture_heap_leaf(id, &value).unwrap();
    assert_eq!(checker.heap_leaf_edges, count);
    let mut invalid = value.clone();
    invalid.ty = hir::Type::Foundation(hir::FoundationType::AllocationFailure);
    assert!(
        checker
            .capture_heap_leaf(id, &invalid)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.heap_leaf_edges, count);
    checker.heap_leaves.clear();
    checker.heap_leaf_edges = 0;
    invalid = value.clone();
    invalid.kind = hir::ExprKind::Local(0);
    assert!(
        checker
            .capture_heap_leaf(id, &invalid)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[id].owner += 1;
    assert!(
        checker
            .capture_heap_leaf(id, &value)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[id].owner -= 1;
    checker.scalar_leaf_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .capture_heap_leaf(id, &value)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.heap_leaves.is_empty());
    assert_eq!(checker.heap_leaf_edges, 0);
}
