use super::*;
use crate::check::dependencies::PointKind;

pub(crate) fn target(source: &str) -> ast::Expr {
    let stmt = crate::parser::parse(source).unwrap().stmts.remove(0);
    let ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    expr
}

#[test]
pub(crate) fn exclusive_index_roots_keep_projected_paths_and_once_only_effects() {
    let mut checker = Checker::new();
    for stmt in crate::parser::parse("r:={->rows:=[{->xs:=[1]}]};n:=0")
        .unwrap()
        .stmts
    {
        checker.stmt(&stmt).unwrap();
    }
    let expr = target("((r).rows)[{n=1;->1}].xs[((1))]");
    let (id, result) = checker
        .with_point_id(PointKind::Expr, expr.span, |checker| {
            checker.exclusive_indexed_points(&expr, expr.span)
        })
        .unwrap();
    let (value, steps) = result.unwrap();
    let hir::ExprKind::ExclusivePath { place, path } = value.kind else {
        panic!()
    };
    assert_eq!(place.fields, [0]);
    assert_eq!(path.len(), 3);
    assert_eq!(steps[1], PathStep::Field(0));
    let mut roots = Vec::new();
    for (step, source) in steps.iter().zip(&path) {
        if let PathStep::Index {
            point,
            capacity,
            span,
        } = step
        {
            let hir::WriteStep::Index(source) = source else {
                panic!()
            };
            let root = &checker.points[*point];
            assert_eq!(root.parent, Some(id));
            assert!(root.complete);
            assert!(root.span.start <= source.index.span.start);
            assert!(root.span.end >= source.index.span.end);
            assert_eq!(*span, source.span);
            assert_eq!(*capacity, 1);
            roots.push(*point);
        }
    }
    assert_eq!(roots.len(), 2);
    assert_ne!(roots[0], roots[1]);
    assert_eq!(
        checker.points[roots[1]].span,
        ast::Span { start: 25, end: 30 }
    );
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| { op.kind == crate::check::dependencies::OperationKind::Write })
            .count(),
        1
    );
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn exclusive_index_roots_preserve_errors_and_restore_active_points() {
    for (source, code) in [
        ("xs[0][1]", "E101"),
        ("xs[1][false]", "E222"),
        ("xs[1][missing]", "E201"),
    ] {
        let mut checker = Checker::new();
        checker
            .stmt(&crate::parser::parse("xs:=[[1]]").unwrap().stmts[0])
            .unwrap();
        let expr = target(source);
        let (id, ()) = checker
            .with_point_id(PointKind::Expr, expr.span, |checker| {
                let prior = checker.point;
                assert_eq!(
                    checker
                        .exclusive_indexed_points(&expr, expr.span)
                        .unwrap_err()
                        .code,
                    code
                );
                assert_eq!(checker.point, prior);
                Ok(())
            })
            .unwrap();
        assert!(checker.points[id].complete);
        assert!(checker.point.is_none());
    }
    for (source, code) in [
        ("xs:[[1]];p:&!(xs[1][1])", "E305"),
        ("xs:=[[1]];p:&!(xs[1][{xs=[[2]];->1}])", "E302"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
}
