use super::{Checker, Kind};
use crate::hir;

pub(crate) fn matcher(stmts: &[hir::Stmt]) -> hir::PointId {
    let hir::Stmt::If { point, .. } = &stmts[0] else {
        panic!()
    };
    point.expect("checked matcher")
}

#[test]
pub(crate) fn hir_matcher_points_preserve_identity_through_clones_and_repeated_spans() {
    let mut checker = Checker::new();
    let stmt = crate::parser::parse("|false|1").unwrap().stmts.remove(0);
    let first = checker.stmt(&stmt).unwrap();
    let second = checker.stmt(&stmt).unwrap();
    let a = matcher(&first);
    let b = matcher(&second);
    assert_ne!(a, b);
    assert_eq!(a, matcher(&first.clone()));
    assert_eq!(b, matcher(&second.clone()));
    assert_eq!(checker.points[a].span, checker.points[b].span);
    assert_ne!(checker.points[a].site, checker.points[b].site);
    for id in [a, b] {
        assert_eq!(checker.points[id].kind, Kind::Match);
        assert!(checker.points[id].complete);
    }
}

#[test]
pub(crate) fn hir_matcher_points_keep_function_and_block_ownership() {
    let source = "|false|1;f:(){|true|2}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let block = crate::parser::parse(source).unwrap();
    let body = checker.block(&block, None, None).unwrap();
    let function = &checker.functions[0].as_ref().unwrap().body;
    let outer = &checker.points[matcher(&body.stmts)];
    let inner = &checker.points[matcher(&function.stmts)];
    assert_ne!(outer.owner, inner.owner);
    assert_eq!(outer.block, Some(body.id));
    assert_eq!(inner.block, Some(function.id));
    let stmt = &checker.points[inner.parent.unwrap()];
    assert_eq!(stmt.kind, Kind::Stmt);
    assert!(stmt.parent.is_none());
}

pub(crate) fn binary(stmts: &[hir::Stmt]) -> Option<hir::PointId> {
    let hir::Stmt::Expr(expr) = &stmts[0] else {
        panic!()
    };
    let hir::ExprKind::Binary { point, .. } = &expr.kind else {
        panic!()
    };
    *point
}

#[test]
pub(crate) fn hir_logic_points_preserve_exact_short_circuit_origins() {
    for (source, kind) in [("false&&true", Kind::And), ("true||false", Kind::Or)] {
        let mut checker = Checker::new();
        let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
        let first = checker.stmt(&stmt).unwrap();
        let second = checker.stmt(&stmt).unwrap();
        let a = binary(&first).unwrap();
        let b = binary(&second).unwrap();
        assert_ne!(a, b);
        assert_eq!(binary(&first.clone()), Some(a));
        assert_eq!(checker.points[a].span, checker.points[b].span);
        assert_eq!(checker.points[a].kind, kind);
        assert_eq!(checker.points[b].kind, kind);
    }
    for source in ["1+2", "1==2"] {
        let mut checker = Checker::new();
        let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
        assert!(binary(&checker.stmt(&stmt).unwrap()).is_none());
    }
}

#[test]
pub(crate) fn hir_logic_points_do_not_invent_origins_for_synthetic_binaries() {
    let mut checker = Checker::new();
    let span = crate::ast::Span::default();
    let value = hir::Expr {
        kind: hir::ExprKind::Bool(false),
        ty: hir::Type::Bool,
        span,
    };
    for kind in [None, Some(Kind::Expr), Some(Kind::And)] {
        let build =
            |checker: &mut Checker| checker.binary_values("||", value.clone(), value.clone(), span);
        let expr = match kind {
            None => build(&mut checker).unwrap(),
            Some(kind) => checker.with_point(kind, span, build).unwrap(),
        };
        assert!(binary(&[hir::Stmt::Expr(expr)]).is_none());
    }
}

#[test]
pub(crate) fn expression_roots_return_outer_ids_through_nested_groups_and_coercions() {
    let stmt = crate::parser::parse("((false&&true))")
        .unwrap()
        .stmts
        .remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    let mut checker = Checker::new();
    let (first, value) = checker.expr_point(&expr, Some(&hir::Type::Bool)).unwrap();
    let (second, _) = checker.expression_point(&expr, None).unwrap();
    assert_ne!(first, second);
    assert_eq!(checker.points[first].span, checker.points[second].span);
    assert!(checker.points[first].parent.is_none());
    assert!(checker.points[second].parent.is_none());
    let hir::ExprKind::Binary {
        point: Some(inner), ..
    } = value.kind
    else {
        panic!()
    };
    let group = checker.points[inner].parent.unwrap();
    assert_eq!(checker.points[group].parent, Some(first));
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn expression_roots_preserve_coercion_and_budget_failure_precedence() {
    let stmt = crate::parser::parse("1+2").unwrap().stmts.remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    let mut checker = Checker::new();
    assert_eq!(
        checker
            .expr_point(&expr, Some(&hir::Type::Bool))
            .unwrap_err()
            .code,
        "E207"
    );
    assert!(!checker.points[0].complete);
    assert!(checker.point.is_none());
    let (_, value) = checker
        .expression_point(&expr, Some(&hir::Type::Bool))
        .unwrap();
    assert_ne!(value.ty, hir::Type::Bool);
    assert!(!checker.flow.spend(usize::MAX));
    let count = checker.points.len();
    let error = checker.expr_point(&expr, None).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation expression budget"));
    assert_eq!(checker.points.len(), count);
}

pub(crate) fn scope_body(body: &hir::Block) -> &hir::Block {
    let hir::Stmt::Expr(expr) = &body.stmts[0] else {
        panic!()
    };
    let hir::ExprKind::Block(block) = &expr.kind else {
        panic!()
    };
    block
}

#[test]
pub(crate) fn hir_exit_sources_preserve_leave_clones_and_repeated_source_spans() {
    let source = crate::parser::parse("'out{'out.leave()}").unwrap();
    let mut checker = Checker::new();
    let mut points = Vec::new();
    for _ in 0..2 {
        let body = checker.block(&source, None, None).unwrap();
        let scope = scope_body(&body);
        let hir::Stmt::Leave {
            target,
            point: Some(point),
        } = scope.stmts[0].clone()
        else {
            panic!()
        };
        assert_eq!(target, scope.id);
        assert_eq!(checker.points[point].block, Some(target));
        assert_eq!(checker.points[point].kind, Kind::Stmt);
        assert!(checker.scope_exits.contains_key(&point));
        points.push(point);
    }
    assert_ne!(points[0], points[1]);
    assert_eq!(
        checker.points[points[0]].span,
        checker.points[points[1]].span
    );
}

#[test]
pub(crate) fn hir_exit_sources_link_restart_ids_without_inventing_seeded_origins() {
    let source = crate::parser::parse("'loop{|false|'loop.restart()}").unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&source, None, None).unwrap();
    let scope = scope_body(&body);
    let hir::Stmt::If { then, .. } = &scope.stmts[0] else {
        panic!()
    };
    let hir::Stmt::Restart { site, target } = then[0].clone() else {
        panic!()
    };
    let input = &checker.restart_inputs[&site];
    assert_eq!(target, input.target);
    assert_eq!(target, scope.id);
    assert_eq!(checker.points[input.point.unwrap()].kind, Kind::Stmt);
    assert!(checker.scope_exits.contains_key(&input.point.unwrap()));
    let mut checker = Checker::new();
    checker
        .track_restart_input(0, 7, crate::ast::Span::default())
        .unwrap();
    assert!(checker.restart_inputs[&0].point.is_none());
}

#[test]
pub(crate) fn composed_roots_preserve_partial_shapes_and_outer_identity() {
    let mut checker = Checker::new();
    let record = hir::Type::Record {
        primary: Box::new(hir::Type::Null),
        fields: vec![
            hir::Field {
                name: "x".into(),
                ty: hir::Type::Bool,
                mutable: false,
            },
            hir::Field {
                name: "y".into(),
                ty: hir::Type::Bool,
                mutable: false,
            },
        ],
    };
    let stmt = crate::parser::parse("(({->x:true}))")
        .unwrap()
        .stmts
        .remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    let (id, value) = checker.composed_point(&expr, record.clone(), None).unwrap();
    assert!(checker.points[id].complete);
    assert!(checker.points[id].parent.is_none());
    let hir::Type::Record { fields, .. } = &value.ty else {
        panic!()
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "x");
    let (next, copy) = checker.composed_point(&expr, record, None).unwrap();
    assert_ne!(id, next);
    assert_eq!(value.ty, copy.ty);
    assert_eq!(checker.points[id].span, checker.points[next].span);
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn composed_roots_keep_errors_and_existing_scalar_fallbacks() {
    let mut checker = Checker::new();
    let record = hir::Type::Record {
        primary: Box::new(hir::Type::Null),
        fields: vec![hir::Field {
            name: "x".into(),
            ty: hir::Type::Bool,
            mutable: false,
        }],
    };
    let stmt = crate::parser::parse("{->x:1}").unwrap().stmts.remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    assert_eq!(
        checker
            .composed_point(&expr, record.clone(), None)
            .unwrap_err()
            .code,
        "E207"
    );
    assert!(checker.point.is_none());
    assert!(!checker.points[0].complete);
    let stmt = crate::parser::parse("true").unwrap().stmts.remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    let (id, value) = checker
        .composed_point(&expr, record, Some(&hir::Type::Bool))
        .unwrap();
    assert_eq!(value.ty, hir::Type::Bool);
    assert!(checker.points[id].complete);
}

#[test]
pub(crate) fn position_roots_retain_exact_outer_ids_without_rechecking_indices() {
    let mut checker = Checker::new();
    let stmt = crate::parser::parse("((1+1))").unwrap().stmts.remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    let (first, value) = checker.list_position_point(&expr, Some(2), 3).unwrap();
    let count = checker.points.len();
    let (second, copy) = checker.list_position_point(&expr, Some(2), 3).unwrap();
    assert_ne!(first, second);
    assert_eq!(checker.points.len(), count * 2);
    assert_eq!(value.ty, copy.ty);
    assert_eq!(checker.points[first].span, expr.span);
    assert!(checker.points[first].complete && checker.points[second].complete);
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn position_roots_preserve_type_one_based_length_and_capacity_errors() {
    for (source, length, capacity, code) in [
        ("false", None, 2, "E222"),
        ("0", None, 2, "E101"),
        ("3", None, 2, "E101"),
        ("2", Some(1), 3, "E101"),
        ("\"name\"", None, 2, "B001"),
    ] {
        let mut checker = Checker::new();
        let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
        let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
            panic!()
        };
        assert_eq!(
            checker
                .list_position_point(&expr, length, capacity)
                .unwrap_err()
                .code,
            code
        );
        assert!(checker.point.is_none());
    }
}

#[test]
pub(crate) fn list_receiver_roots_keep_shared_dereference_and_exact_outer_identity() {
    let mut checker = Checker::new();
    super::super::tests::statements(&mut checker, "xs:[1,2];view:&xs");
    for (source, borrowed) in [("((xs))", false), ("((view))", true)] {
        let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
        let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
            panic!()
        };
        let before = checker.points.len();
        let (first, value) = checker.list_receiver_point(&expr).unwrap();
        let count = checker.points.len() - before;
        let (second, copy) = checker.list_receiver_point(&expr).unwrap();
        assert_ne!(first, second);
        assert_eq!(checker.points.len() - before, count * 2);
        assert_eq!(checker.points[first].span, expr.span);
        assert!(checker.points[first].complete);
        assert!(matches!(value.ty, hir::Type::List { capacity: 2, .. }));
        assert_eq!(matches!(value.kind, hir::ExprKind::Deref(_)), borrowed);
        assert_eq!(value.ty, copy.ty);
        assert!(checker.point.is_none());
    }
}

#[test]
pub(crate) fn list_receiver_roots_preserve_never_and_ordinary_errors() {
    let mut checker = Checker::new();
    super::super::tests::statements(&mut checker, "d:@\"debug\"");
    let stmt = crate::parser::parse("d.panic(\"stop\")")
        .unwrap()
        .stmts
        .remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    let (point, value) = checker.list_receiver_point(&expr).unwrap();
    assert_eq!(value.ty, hir::Type::Never);
    assert!(checker.points[point].complete);
    assert!(checker.point.is_none());
    let stmt = crate::parser::parse("missing").unwrap().stmts.remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    assert_eq!(checker.list_receiver_point(&expr).unwrap_err().code, "E201");
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn element_parent_roots_distinguish_places_views_and_statement_temporaries() {
    let mut checker = Checker::new();
    super::super::tests::statements(&mut checker, "xs:[1];view:&xs");
    for (source, kind) in [("((xs))", 0), ("view", 1), ("[1]", 2)] {
        let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
        let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
            panic!()
        };
        checker.statement.push((17, false));
        let (point, value) = checker.element_parent(&expr, expr.span).unwrap();
        assert_eq!(checker.points[point].span, expr.span);
        assert!(checker.points[point].complete);
        assert!(matches!(value.ty, hir::Type::Reference(_)));
        match (kind, value.kind) {
            (0, hir::ExprKind::Borrow(_)) | (1, hir::ExprKind::Local(_)) => {}
            (2, hir::ExprKind::TemporaryBorrow { id, statement, .. }) => {
                assert_eq!(statement, 17);
                assert_eq!(checker.proofs.temporaries[&id], 17);
            }
            _ => panic!("wrong parent representation"),
        }
        assert_eq!(checker.statement.pop().unwrap().1, kind == 2);
        assert!(checker.point.is_none());
    }
    crate::compile("rows:[{->xs:[1]}];n:*(&(rows[1].xs[1]))").unwrap();
}

#[test]
pub(crate) fn borrowed_roots_keep_distinct_ids_and_restore_parents_after_errors() {
    let mut checker = Checker::new();
    super::super::tests::statements(&mut checker, "xs:[1]");
    let stmt = crate::parser::parse("((xs))").unwrap().stmts.remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    let (a, _) = checker.borrowed_point(&expr, expr.span).unwrap();
    let (b, _) = checker.borrowed_point(&expr, expr.span).unwrap();
    assert_ne!(a, b);
    assert_eq!(checker.points[a].span, checker.points[b].span);
    let stmt = crate::parser::parse("missing").unwrap().stmts.remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    assert_eq!(
        checker.borrowed_point(&expr, expr.span).unwrap_err().code,
        "E201"
    );
    assert!(checker.point.is_none());
}
