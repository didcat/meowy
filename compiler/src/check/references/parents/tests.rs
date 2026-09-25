use super::super::dereference::{expr, prefix};
use super::*;
use crate::check::dependencies::PointKind;

#[test]
pub(crate) fn projected_parent_roots_keep_value_reference_and_indexed_selection() {
    let mut checker = Checker::new();
    prefix(&mut checker, "r:{->n:1};p:&r;rows:[{->n:2}]");
    for source in ["r", "p", "*((p))", "rows[1]"] {
        let root = expr(source);
        let (outer, (point, temporary, value)) = checker
            .with_point_id(PointKind::Expr, root.span, |checker| {
                checker.projected_parent(&root)
            })
            .unwrap();
        assert_eq!(checker.points[point].parent, Some(outer));
        assert_eq!(temporary, None);
        assert!(checker.points[point].complete);
        if source == "*((p))" {
            let ast::ExprKind::Unary { value, .. } = &root.kind else {
                panic!()
            };
            assert_eq!(checker.points[point].span, value.span);
            assert!(checker.derefs.is_empty());
        } else {
            assert_eq!(checker.points[point].span, root.span);
        }
        if source == "rows[1]" {
            assert!(matches!(value.kind, hir::ExprKind::ElementBorrow { .. }));
            assert!(checker.elements.contains_key(&point));
        } else {
            assert!(matches!(value.kind, hir::ExprKind::Local(_)));
        }
        assert!(checker.point.is_none());
    }
    assert!(checker.proofs.temporaries.is_empty());
    crate::compile("rows:[{->n:1}];q:&(rows[1].n);x:*q").unwrap();
}

#[test]
pub(crate) fn projected_parent_roots_preserve_temporary_statement_identity_and_once_only_effects() {
    let mut checker = Checker::new();
    prefix(&mut checker, "n:=0");
    let root = expr("({n=2;->x:1})");
    checker.statement.push((17, false));
    let (point, temporary, value) = checker.projected_parent(&root).unwrap();
    let hir::ExprKind::TemporaryBorrow {
        id,
        statement,
        value,
    } = value.kind
    else {
        panic!()
    };
    assert_eq!(statement, 17);
    assert_eq!(temporary, Some((id, statement)));
    assert_eq!(checker.proofs.temporaries[&id], 17);
    assert_eq!(checker.locals[id], value.ty);
    assert_eq!(checker.points[point].span, root.span);
    assert!(checker.statement.pop().unwrap().1);
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| { op.kind == crate::check::dependencies::OperationKind::Write })
            .count(),
        1
    );
    crate::compile("n:=0;x:*(&(({n=2;->x:1}).x))").unwrap();
}

#[test]
pub(crate) fn projected_parent_roots_keep_returned_owners_and_reference_fields() {
    let mut checker = Checker::new();
    prefix(&mut checker, "<R>:<{n<int32>}>;make<R>:(){->n:1}");
    let root = expr("make()");
    checker.statement.push((19, false));
    let (point, temporary, value) = checker.projected_parent(&root).unwrap();
    assert!(temporary.is_some());
    let hir::ExprKind::TemporaryBorrow { value, .. } = value.kind else {
        panic!()
    };
    let hir::ExprKind::Call { site, .. } = value.kind else {
        panic!()
    };
    assert_eq!(checker.invocations[&site].point, point);
    assert_eq!(checker.invocations.len(), 1);
    assert!(checker.statement.pop().unwrap().1);
    for source in [
        "<R>:<{n<int32>}>;make<R>:(){->n:1};x:*(&(make().n))",
        "r:{->n:1};holder:{->p:&r};q:&(holder.p.n);x:*q",
        "rows:[{->xs:[{->n:1}]}];q:&(rows[1].xs[1].n);x:*q",
    ] {
        crate::compile(source).unwrap();
    }
}

#[test]
pub(crate) fn projected_parent_roots_preserve_errors_stopped_inputs_and_budget_restoration() {
    for (source, code) in [
        ("missing", "E201"),
        ("rows[0]", "E101"),
        ("rows[false]", "E222"),
    ] {
        let mut checker = Checker::new();
        prefix(&mut checker, "rows:[{->n:1}]");
        let root = expr(source);
        checker
            .with_point_id(PointKind::Expr, root.span, |checker| {
                let prior = checker.point;
                assert_eq!(checker.projected_parent(&root).unwrap_err().code, code);
                assert_eq!(checker.point, prior);
                Ok(())
            })
            .unwrap();
        assert!(checker.point.is_none());
    }
    let mut checker = Checker::new();
    prefix(
        &mut checker,
        r#"d:@"debug";stop<never>:(){d.panic("stop")}"#,
    );
    let root = expr("stop()");
    let (point, temporary, value) = checker.projected_parent(&root).unwrap();
    assert_eq!(temporary, None);
    assert_eq!(value.ty, Type::Never);
    assert_eq!(checker.invocations.values().next().unwrap().point, point);
    assert!(checker.proofs.temporaries.is_empty());
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(checker.projected_parent(&root).unwrap_err().code, "B001");
    assert!(checker.point.is_none());
    for (source, code) in [
        ("p:&({->n:1}.n);x:*p", "E303"),
        (
            "rows:=[{->n:1}];p:&(rows[1].n);rows[1]={->n:2};x:*p",
            "E302",
        ),
        ("r:{->n:1};p:&r;q:&(p.missing)", "E201"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
}

#[test]
pub(crate) fn projected_parent_temporaries_distinguish_materialization_from_existing_references() {
    let mut checker = Checker::new();
    checker.statement.push((23, false));
    let root = expr("{->n:1}");
    let (_, temporary, value) = checker.projected_parent(&root).unwrap();
    let hir::ExprKind::TemporaryBorrow { id, statement, .. } = value.kind else {
        panic!()
    };
    assert_eq!(temporary, Some((id, statement)));
    let root = expr("&({->n:2})");
    let (_, temporary, value) = checker.projected_parent(&root).unwrap();
    let hir::ExprKind::TemporaryBorrow {
        id: existing,
        statement,
        ..
    } = value.kind
    else {
        panic!()
    };
    assert_eq!(temporary, None);
    assert_ne!(id, existing);
    assert_eq!(statement, 23);
    assert_eq!(checker.proofs.temporaries[&existing], statement);
    assert_eq!(checker.proofs.temporaries.len(), 2);
    assert!(checker.statement.pop().unwrap().1);
}
