use super::*;

pub(crate) fn expr(source: &str) -> ast::Expr {
    let ast::StmtKind::Expr(expr) = crate::parser::parse(source).unwrap().stmts.remove(0).kind
    else {
        panic!()
    };
    expr
}

pub(crate) fn setup(source: &str) -> Checker {
    let mut checker = Checker::new();
    for stmt in crate::parser::parse(source).unwrap().stmts {
        checker.stmt(&stmt).unwrap();
    }
    checker
}

pub(crate) fn child(checker: &Checker, id: hir::PointId) -> hir::PointId {
    let children = checker
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.parent == Some(id))
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(children.len(), 1);
    children[0]
}

#[test]
pub(crate) fn shared_context_roots_separate_raw_operations_and_keep_caller_identity() {
    let ty = Type::Reference(Box::new(Type::Int {
        signed: true,
        bits: 32,
    }));
    let mut checker = setup("id<&!int32>:(p<&!int32>){->p};n:=1;p:&!n");
    for source in ["&!*p", "id(&!n)", "&n", "((p))"] {
        let expr = expr(source);
        let (first, value) = checker.expr_point(&expr, Some(&ty)).unwrap();
        let (second, _) = checker.expr_point(&expr, Some(&ty)).unwrap();
        assert_ne!(first, second);
        assert_eq!(checker.points[first].span, checker.points[second].span);
        assert_eq!(checker.points[first].kind, PointKind::Expr);
        assert!(checker.points[first].parent.is_none());
        assert_eq!(value.ty, ty);
        let raw = child(&checker, first);
        assert_eq!(checker.points[raw].span, expr.span);
        assert!(checker.points[raw].complete);
        match source {
            "&!*p" => assert_eq!(
                checker.reborrow_ops[&raw].mode,
                hir::ReferenceMode::Exclusive
            ),
            "id(&!n)" => assert!(checker.invocations.values().any(|call| call.point == raw)),
            "&n" => assert!(checker.place_borrows.contains_key(&raw)),
            "((p))" => {
                let inner = child(&checker, raw);
                assert_eq!(checker.points[inner].kind, PointKind::Expr);
                assert!(checker.region_edges.contains_key(&raw));
            }
            _ => unreachable!(),
        }
    }
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn shared_context_roots_keep_branch_query_read_and_function_ancestry() {
    let source = "p:@\"proof\";n:3;f<&int32>:(v<&int32>){q:p.can_copy<({-><uint8[n]>})>();->v};x:1;r<&int32>:{|false|0;->f(&x)}";
    let mut checker = Checker::new();
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    let query = &checker.queries[0];
    let read = checker.body_inputs.values().flatten().next().unwrap();
    assert_eq!(checker.points[read.point].parent, Some(query.point));
    assert_ne!(checker.points[query.point].owner, 0);
    assert_eq!(
        checker.points[checker.points[query.point].parent.unwrap()].kind,
        PointKind::Stmt
    );
    for call in checker.invocations.values() {
        let outer = checker.points[call.point].parent.unwrap();
        assert_eq!(checker.points[outer].owner, call.owner);
        assert_eq!(
            checker.points[outer].block,
            checker.points[call.point].block
        );
        assert_eq!(checker.points[outer].site, checker.points[call.point].site);
    }
    assert!(!checker.branch_edges.is_empty());
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
}

#[test]
pub(crate) fn shared_context_roots_preserve_stops_diagnostics_and_budget_restoration() {
    let ty = Type::Reference(Box::new(Type::Int {
        signed: true,
        bits: 32,
    }));
    let mut checker = setup(r#"d:@"debug""#);
    let (outer, value) = checker
        .expr_point(&expr(r#"d.panic("stop")"#), Some(&ty))
        .unwrap();
    assert_eq!(value.ty, Type::Never);
    assert!(checker.outputs.contains_key(&child(&checker, outer)));
    assert_eq!(checker.reborrows, 0);
    for (source, code) in [("missing", "E201"), ("1", "E207"), ("false&&true", "E207")] {
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .expr_point(&expr(source), Some(&ty))
                .unwrap_err()
                .code,
            code
        );
        assert!(!checker.points[0].complete);
        assert!(checker.point.is_none());
        assert_eq!(checker.reborrows, 0);
        if source == "false&&true" {
            assert_eq!(checker.points[child(&checker, 0)].kind, PointKind::And);
            assert_eq!(checker.points[0].kind, PointKind::Expr);
        }
    }
    let mut checker = Checker::new();
    assert!(!checker.flow.spend(usize::MAX));
    let error = checker.expr_point(&expr("missing"), Some(&ty)).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation expression budget"));
    assert!(checker.points.is_empty());
    assert!(checker.point.is_none());
}
