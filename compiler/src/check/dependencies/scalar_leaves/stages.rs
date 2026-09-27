use super::{
    tests::{expr, setup},
    *,
};

#[test]
pub(crate) fn scalar_leaf_stages_precede_expected_conversion_without_group_bypasses() {
    let mut checker = Checker::new();
    let ty = hir::Type::union(vec![
        hir::Type::Int {
            bits: 32,
            signed: true,
        },
        hir::Type::Null,
    ]);
    let (outer, _) = checker.expr_point(&expr("7"), Some(&ty)).unwrap();
    let input = checker.coercions[&outer].input;
    assert_eq!(
        checker.scalar_leaves[&input].edges,
        [
            Edge::new(Port::Entry(input), Port::Operation(input), Route::Next),
            Edge::new(Port::Operation(input), Port::Normal(input), Route::Next),
        ]
    );
    assert!(checker.coercions[&outer].edges.contains(&Edge::new(
        Port::Normal(input),
        Port::Operation(outer),
        Route::Next
    )));
    let mut checker = Checker::new();
    let (outer, _) = checker.expr_point(&expr("((true))"), None).unwrap();
    let (&leaf, _) = checker.scalar_leaves.first_key_value().unwrap();
    assert_ne!(leaf, outer);
    assert_eq!(checker.scalar_leaves.len(), 1);
    let inner = checker.points[leaf].parent.unwrap();
    assert!(checker.region_edges[&inner].contains(&Edge::new(
        Port::Normal(leaf),
        Port::Normal(inner),
        Route::Next
    )));
}

#[test]
pub(crate) fn scalar_leaf_stages_preserve_formatting_required_work_and_probe_purity() {
    let checker = setup("d:@\"debug\";d.print(\"before {7} after\")");
    assert_eq!(checker.scalar_leaves.len(), 1);
    let (&id, leaf) = checker.scalar_leaves.first_key_value().unwrap();
    assert!(matches!(leaf.kind, Kind::Int { .. }));
    let output = checker.outputs.values().next().unwrap();
    assert!(output.parts[0].is_none() && output.parts[2].is_none());
    assert_eq!(output.parts[1].unwrap().point, id);
    let checker = setup("<T>:{n<int32>:1+2;-><uint8[n]>}");
    assert!(checker.scalar_leaves.is_empty());
    let mut checker = Checker::new();
    let before = checker.points.len();
    let (_, value) = checker
        .with_point_id(PointKind::Expr, Span::default(), |checker| {
            checker.integer("7", false, None, Span::default())
        })
        .unwrap();
    assert_eq!(checker.points.len(), before + 1);
    assert!(matches!(value.kind, hir::ExprKind::Int(7)));
    assert!(checker.scalar_leaves.is_empty());
}

#[test]
pub(crate) fn scalar_leaf_stages_keep_function_owners_control_and_existing_errors() {
    let source = "flag:false;|flag|x:1;f<int8>:(){->-128}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert!(checker.scalar_leaves.values().any(|leaf| leaf.control));
    assert!(checker.scalar_leaves.values().any(|leaf| leaf.owner != 0));
    for (source, code) in [
        ("x<int8>:128", "E216"),
        ("x<uint8>:-1", "E222"),
        ("x<boolean>:1", "E207"),
        ("x:\"text {1}\"", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn scalar_leaf_stages_validate_shapes_and_publish_shared_budgets_atomically() {
    let mut checker = Checker::new();
    let (id, value) = checker.expr_point(&expr("7"), None).unwrap();
    let count = checker.scalar_leaf_edges;
    checker.capture_scalar_leaf(id, &value).unwrap();
    assert_eq!(checker.scalar_leaf_edges, count);
    let mut invalid = value.clone();
    invalid.ty = hir::Type::Bool;
    assert!(
        checker
            .capture_scalar_leaf(id, &invalid)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.scalar_leaf_edges, count);
    checker.scalar_leaves.clear();
    checker.scalar_leaf_edges = 0;
    checker.points[id].owner += 1;
    assert!(
        checker
            .capture_scalar_leaf(id, &value)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[id].owner -= 1;
    checker.local_read_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .capture_scalar_leaf(id, &value)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.scalar_leaves.is_empty());
    assert_eq!(checker.scalar_leaf_edges, 0);
}
