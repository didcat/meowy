use super::*;

pub(crate) fn expr(source: &str) -> crate::ast::Expr {
    let crate::ast::StmtKind::Expr(expr) =
        crate::parser::parse(source).unwrap().stmts.remove(0).kind
    else {
        panic!()
    };
    expr
}

pub(crate) fn prepare(source: &str) -> Checker {
    let mut checker = Checker::new();
    for stmt in crate::parser::parse(source).unwrap().stmts {
        checker.stmt(&stmt).unwrap();
    }
    checker
}

#[test]
pub(crate) fn record_sequences_keep_exact_ordered_roots_through_groups_and_repeated_spans() {
    for source in ["((a))==((b))", "a!=b"] {
        let mut checker = prepare("a:{->n:1};b:{->n:2}");
        let expr = expr(source);
        let crate::ast::ExprKind::Binary { left, right, .. } = &expr.kind else {
            panic!()
        };
        let mut prior = None;
        for _ in 0..2 {
            let (root, value) = checker.expr_point(&expr, None).unwrap();
            assert_ne!(prior, Some(root));
            prior = Some(root);
            assert_eq!(value.ty, hir::Type::Bool);
            let seq = &checker.sequences[&Source::Expr(root)];
            let a = seq.items[0].unwrap();
            let b = seq.items[1].unwrap();
            assert_eq!(
                seq.edges,
                [Edge::new(Port::Normal(a), Port::Entry(b), Route::Next)]
            );
            for (id, span) in [(a, left.span), (b, right.span)] {
                assert_eq!(checker.points[id].parent, Some(root));
                assert_eq!(checker.points[id].span, span);
                assert!(checker.points[id].complete);
            }
        }
    }
    let (checker, _) = super::tests::check("f<boolean>:(a<{n<int32>}>,b<{n<int32>}>){->a==b}");
    assert!(
        checker
            .sequences
            .iter()
            .any(|(source, seq)| matches!(source, Source::Expr(_))
                && seq.owner != 0
                && seq.items.iter().all(Option::is_some))
    );
}

#[test]
pub(crate) fn record_sequences_check_calls_and_partial_blocks_once_without_return_bypasses() {
    let setup = "n:=0;f<{n<int32>}>:(){->{->n:1}}";
    let source = "f()==({n=1;->f()})";
    crate::compile(&format!("{setup};same:{source}")).unwrap();
    let mut checker = prepare(setup);
    let (root, _) = checker.expr_point(&expr(source), None).unwrap();
    let seq = &checker.sequences[&Source::Expr(root)];
    assert_eq!(checker.invocations.len(), 2);
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| op.kind == super::super::OperationKind::Write)
            .count(),
        1
    );
    let left = seq.items[0].unwrap();
    let call = checker
        .invocations
        .values()
        .find(|call| checker.points[call.point].parent == Some(left))
        .unwrap();
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    assert!(!checker.region_edges.contains_key(&left));
    assert!(!checker.endpoints.contains_key(&Source::Expr(left)));
    let right = seq.items[1].unwrap();
    let Port::Entry(child) = checker.region_edges[&right][0].to else {
        panic!()
    };
    assert!(checker.endpoints.contains_key(&Source::Expr(child)));
    let mut checker = prepare("a:{->n:1};d:@\"debug\";stop<never>:(){d.panic(\"stop\")}");
    let (root, value) = checker.expr_point(&expr("a==stop()"), None).unwrap();
    assert_eq!(value.ty, hir::Type::Bool);
    let hir::ExprKind::Binary { right, .. } = &value.kind else {
        panic!()
    };
    let hir::ExprKind::Coerce { value: stopped } = &right.kind else {
        panic!()
    };
    assert_eq!(stopped.ty, hir::Type::Never);
    let seq = &checker.sequences[&Source::Expr(root)];
    assert!(seq.items.iter().all(Option::is_some));
    assert!(!checker.invocations.values().next().unwrap().may_return);
}

#[test]
pub(crate) fn record_sequences_preserve_contextual_primary_projection_and_errors() {
    let setup = "n<uint8>:7;a:{->n;->tag:true}";
    for source in ["a==7", "7!=a", "a=={->7}", "{->7}==a"] {
        crate::compile(&format!("{setup};same:{source}")).unwrap();
        let mut checker = prepare(setup);
        let (root, value) = checker.expr_point(&expr(source), None).unwrap();
        let hir::ExprKind::Binary { left, right, .. } = value.kind else {
            panic!()
        };
        assert_eq!(
            left.ty,
            hir::Type::Int {
                signed: false,
                bits: 8
            }
        );
        assert_eq!(left.ty, right.ty);
        assert!(
            checker.sequences[&Source::Expr(root)]
                .items
                .iter()
                .all(Option::is_some)
        );
    }
    for (source, code) in [
        ("a==256", "E216"),
        ("a==false", "E222"),
        ("a=={->7;->tag:1}", "E207"),
    ] {
        let mut checker = prepare(setup);
        assert_eq!(
            checker.expr_point(&expr(source), None).unwrap_err().code,
            code,
            "{source}"
        );
        assert!(checker.point.is_none());
    }
}

#[test]
pub(crate) fn record_sequences_use_shared_edge_budgets_without_partial_publication() {
    let mut checker = prepare("a:{->n:1}");
    let count = checker.sequences.len();
    checker.dispatch_edges = super::super::edges::MAX_EDGES;
    let error = checker.expr_point(&expr("a==a"), None).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("coercion-operation budget"));
    assert!(checker.coercions.is_empty());
    assert_eq!(checker.sequences.len(), count);
    assert!(checker.point.is_none());
}
