use super::{
    tests::{expr, prepare},
    *,
};

#[test]
pub(crate) fn expected_stages_distinguish_forwarding_primary_and_conversion_after_raw_effects() {
    let int = hir::Type::Int {
        bits: 32,
        signed: true,
    };
    let union = hir::Type::union(vec![int.clone(), hir::Type::Null]);
    let mut checker = prepare("<Choice>:<int32><null>;n:7;r:{->n;->tag:true};f<int32>:(){->1}");
    for (source, target, primary, kind) in [
        ("n", &int, false, Kind::Forward),
        ("n", &union, false, Kind::Convert),
        ("r", &int, true, Kind::Forward),
        ("r", &union, true, Kind::Convert),
        ("f()", &union, false, Kind::Convert),
        ("1+2", &union, false, Kind::Convert),
        ("n~<Choice>", &union, false, Kind::Forward),
        ("false||true", &hir::Type::Bool, false, Kind::Forward),
    ] {
        let (id, value) = checker.expr_point(&expr(source), Some(target)).unwrap();
        let op = &checker.coercions[&id];
        assert_eq!((op.primary, op.kind), (primary, kind), "{source}");
        assert_eq!(value.ty, *target);
        assert_eq!(
            op.edges[0],
            Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next)
        );
        assert_eq!(op.edges.last().unwrap().to, Port::Normal(id));
        assert_eq!(
            op.edges.len(),
            2 + usize::from(primary) + usize::from(kind == Kind::Convert)
        );
        assert!(!checker.region_edges.contains_key(&id));
        if primary {
            assert_eq!(
                op.edges[1],
                Edge::new(
                    Port::Normal(op.input),
                    Port::Projection { point: id, step: 0 },
                    Route::Next
                )
            );
        }
        if source == "f()" {
            assert!(
                checker
                    .invocations
                    .values()
                    .any(|call| call.point == op.input && call.may_return)
            );
        }
        if source == "1+2" {
            assert_eq!(
                checker.binaries[&op.input].edges.last().unwrap().route,
                Route::Checked
            );
        }
        if source == "false||true" {
            assert!(checker.branch_edges.contains_key(&op.input));
        }
    }
}

#[test]
pub(crate) fn expected_stages_keep_direct_and_projected_never_distinct() {
    let mut checker = prepare("d:@\"debug\"");
    let (id, value) = checker
        .expr_point(&expr("d.panic(\"stop\")"), Some(&hir::Type::Bool))
        .unwrap();
    assert_eq!(value.ty, hir::Type::Never);
    assert_eq!(checker.coercions[&id].kind, Kind::Stopped);
    assert!(!checker.coercions[&id].primary);
    assert_eq!(checker.coercions[&id].edges.len(), 1);
    assert!(checker.reborrow_ops.is_empty());
    let source = "f<boolean>:(r<{-><never>;tag<boolean>}>){->r}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    let (&id, op) = checker.coercions.iter().find(|(_, op)| op.primary).unwrap();
    assert_eq!(op.kind, Kind::Stopped);
    assert_eq!(
        op.edges,
        [
            Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next),
            Edge::new(
                Port::Normal(op.input),
                Port::Projection { point: id, step: 0 },
                Route::Next
            ),
        ]
    );
    assert_ne!(op.owner, 0);
}

#[test]
pub(crate) fn expected_stages_preserve_shared_reborrows_required_work_and_control() {
    let int = hir::Type::Int {
        bits: 32,
        signed: true,
    };
    let shared = hir::Type::Reference(Box::new(int.clone()));
    let mut checker = prepare("n:=1;p:&!n");
    let (id, _) = checker.expr_point(&expr("p"), Some(&shared)).unwrap();
    assert!(checker.reborrow_ops.contains_key(&id));
    assert!(!checker.coercions.contains_key(&id));
    let mut checker = prepare("n:1;r:{->&n;->tag:0}");
    let (id, _) = checker.expr_point(&expr("r"), Some(&shared)).unwrap();
    assert!(checker.coercions[&id].primary);
    assert!(checker.reborrow_ops.is_empty());
    let mut checker = Checker::new();
    checker.required = true;
    checker.expr_point(&expr("1+2"), Some(&int)).unwrap();
    assert!(checker.coercions.is_empty());
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;|flag|x<int32><null>:1").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.coercions.values().any(|op| op.control));
}

#[test]
pub(crate) fn expected_stages_validate_projection_identity_and_shared_budgets_atomically() {
    let int = hir::Type::Int {
        bits: 32,
        signed: true,
    };
    let mut checker = prepare("r:{->1;->tag:true}");
    let (id, _) = checker.expr_point(&expr("r"), Some(&int)).unwrap();
    let op = checker.coercions[&id].clone();
    let count = checker.coercion_edges;
    checker
        .coercion_stages(id, op.input, op.kind, true, op.span)
        .unwrap();
    assert_eq!(checker.coercion_edges, count);
    assert!(
        checker
            .coercion_stages(id, op.input, op.kind, false, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.coercions.clear();
    checker.coercion_edges = 0;
    checker.binary_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .coercion_stages(id, op.input, op.kind, true, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.coercions.is_empty());
    assert_eq!(checker.coercion_edges, 0);
    let mut checker = Checker::new();
    assert_eq!(
        checker
            .expr_point(&expr("1+2"), Some(&hir::Type::Bool))
            .unwrap_err()
            .code,
        "E207"
    );
    assert!(checker.coercions.is_empty());
}
