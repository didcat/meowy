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
    assert!(inner.parent.is_none());
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
