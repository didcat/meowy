use super::super::roots::{expr, setup};
use super::*;
use crate::check::dependencies::SequenceSource;

#[test]
pub(crate) fn dispatch_roots_keep_receiver_source_local_and_body_identities() {
    let mut checker = setup("n:=2;r:{->n:3};f<int32>:(){->4}");
    let ty = Type::Int {
        signed: true,
        bits: 32,
    };
    for source in [
        "((n)).{->$}",
        "f().{->$}",
        "r.{->$.n}",
        "(&n).{->*$}",
        "({n=n+1;->n}).{->$}",
    ] {
        let expr = expr(source);
        let ExprKind::DispatchBlock { value, block } = &expr.kind else {
            panic!()
        };
        let scopes = checker.scopes.len();
        let frames = checker.frames.len();
        let (outer, (input, local, body)) = checker
            .with_point_id(PointKind::Expr, expr.span, |checker| {
                checker.dispatch_point(value, block, Some(&ty), expr.span)
            })
            .unwrap();
        assert_eq!(checker.points[input].parent, Some(outer));
        assert_eq!(checker.points[input].span, value.span);
        assert!(checker.points[input].complete);
        assert_eq!(checker.bodies[&body.id].parent, Some(outer));
        let hir::Stmt::Bind {
            id,
            value: receiver,
        } = &body.stmts[0]
        else {
            panic!()
        };
        assert_eq!(*id, local);
        assert_eq!(checker.locals[local], receiver.ty);
        assert!(checker.proofs.receivers.contains(&local));
        assert!(checker.proofs.dispatches.contains(&body.id));
        assert_eq!(
            checker.sequences[&SequenceSource::Block(body.id)].items[0],
            None
        );
        assert_eq!(body.ty, ty);
        if matches!(value.kind, ExprKind::Call { .. }) {
            assert!(checker.invocations.values().any(|call| call.point == input));
        }
        assert_eq!(checker.scopes.len(), scopes);
        assert_eq!(checker.frames.len(), frames);
    }
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| op.kind == crate::check::dependencies::OperationKind::Write)
            .count(),
        1
    );
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn dispatch_roots_keep_nested_binding_and_stopped_receiver_semantics() {
    let source = "v:2.{outer:$;inner:3.{->$};->{->$+outer+inner}}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert_eq!(checker.proofs.receivers.len(), 2);
    let mut checker = setup(r#"d:@"debug";stop<never>:(){d.panic("stop")}"#);
    let expr = expr("stop().{}");
    let ExprKind::DispatchBlock { value, block } = &expr.kind else {
        panic!()
    };
    let (_, (input, local, body)) = checker
        .with_point_id(PointKind::Expr, expr.span, |checker| {
            checker.dispatch_point(value, block, None, expr.span)
        })
        .unwrap();
    assert_eq!(checker.locals[local], Type::Never);
    assert_eq!(body.ty, Type::Never);
    assert_eq!(body.stmts.len(), 1);
    assert!(
        checker
            .invocations
            .values()
            .any(|call| call.point == input && !call.may_return)
    );
    assert_eq!(crate::compile("v:3.{->&$}").unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn dispatch_roots_preserve_gates_errors_and_budget_restoration() {
    for (source, code) in [
        ("missing.{}", "E201"),
        ("(&!n).{}", "B001"),
        ("n.{$=4}", "E305"),
        ("n.{->missing}", "E201"),
    ] {
        let mut checker = setup("n:=2");
        let expr = expr(source);
        assert_eq!(checker.expr_point(&expr, None).unwrap_err().code, code);
        assert!(checker.point.is_none());
        assert!(!checker.control);
    }
    let mut checker = Checker::new();
    assert!(!checker.flow.spend(usize::MAX));
    let expr = expr("3.{}");
    let ExprKind::DispatchBlock { value, block } = &expr.kind else {
        panic!()
    };
    let error = checker
        .dispatch_point(value, block, None, expr.span)
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation expression budget"));
    assert!(checker.proofs.receivers.is_empty());
    assert!(checker.points.is_empty());
}
