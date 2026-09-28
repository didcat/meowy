use super::{tests::checked, *};
use crate::{ast, check::dependencies::SequenceSource};

pub(super) fn opened(source: &str) -> (Checker, ast::Block) {
    let parsed = crate::parser::parse(source).unwrap();
    let ast::StmtKind::Expr(ast::Expr {
        kind: ast::ExprKind::Block(body),
        ..
    }) = &parsed.stmts[0].kind
    else {
        panic!("labeled block")
    };
    let mut checker = Checker::new();
    checker.block_start(body, None, None, false).unwrap();
    (checker, body.clone())
}

#[test]
pub(crate) fn control_alias_endpoints_preserve_targets_flags_and_normal_completion() {
    let (mut checker, ast) = opened("'out{finish:'out.leave;again:'out.restart;copy:(again)}");
    let target = checker.frames.last().unwrap().id;
    let reach = checker.reach;
    for (stmt, name, restart) in [
        (&ast.stmts[0], "finish", false),
        (&ast.stmts[1], "again", true),
        (&ast.stmts[2], "copy", true),
    ] {
        let (id, stmts) = checker.checked_stmt(stmt).unwrap();
        assert!(stmts.is_empty());
        let value = checker.value(name, stmt.span).unwrap();
        assert!(
            matches!(value, Value::Control { target: found, owner: 0, restart: flag } if found == target && flag == restart)
        );
        assert_eq!(
            BindingIdentity::capture(&value),
            Some(BindingIdentity::Control { target, owner: 0 })
        );
        assert_eq!(
            checker.endpoints[&SequenceSource::Stmt(id)],
            [Edge::new(Port::Entry(id), Port::Normal(id), Route::Next)]
        );
    }
    assert_eq!(checker.reach, reach);
    assert!(checker.scope_exits.is_empty());
    assert!(checker.restart_edges.is_empty());
    assert_eq!(checker.restarts, 0);
    assert!(checker.locals.is_empty());
    crate::compile(
        "f:(){'out{finish:'out.leave;'inner{copy:finish;x:1}}};row:{->leave:7};copy:row.leave",
    )
    .unwrap();
}

#[test]
pub(crate) fn control_alias_endpoints_leave_real_exit_and_restart_edges_to_calls() {
    for (operation, continued) in [("leave", true), ("restart", false)] {
        let source = format!("'out{{action:'out.{operation};copy:action;copy();x:1}};y:2");
        crate::compile(&source).unwrap();
        let (mut checker, body) = checked(&source);
        assert_eq!(checker.scope_exits.len(), 1);
        let (&call, exit) = checker.scope_exits.iter().next().unwrap();
        let target = match exit.edge.to {
            Port::Leave(target) | Port::Restart { target, .. } => target,
            _ => panic!("scope exit"),
        };
        let items = checker.sequences[&SequenceSource::Block(target)]
            .items
            .clone();
        assert_eq!(items[2], Some(call));
        assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(call)));
        let last = checker.sequences[&SequenceSource::Block(body.id)]
            .items
            .last()
            .unwrap()
            .unwrap();
        let graph = checker.forward_index(Span::default()).unwrap();
        let walk = graph
            .walk(
                Port::BlockEntry(body.id),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert!(walk.ports.contains(&Port::Normal(items[1].unwrap())));
        assert!(!walk.ports.contains(&Port::Operation(items[3].unwrap())));
        assert_eq!(walk.ports.contains(&Port::Operation(last)), continued);
        assert_eq!(walk.backedges.len(), usize::from(!continued));
    }
}

#[test]
pub(crate) fn control_alias_endpoints_keep_scope_capture_mutability_and_call_errors() {
    for (source, code) in [
        ("alias:'missing.leave", "E201"),
        ("'out{alias:'out.leave;alias:'out.restart}", "E203"),
        ("'out{alias:='out.leave}", "B001"),
        ("'out{alias<int32>:'out.leave}", "B001"),
        ("'out{alias:'out.leave;f:(){copy:alias}}", "B001"),
        ("'out{f:(){alias:'out.leave}}", "E201"),
        ("'out{alias:'out.leave;alias(1)}", "E212"),
        ("'out{alias:'out.restart;alias(1)}", "E212"),
        ("'out{alias:'out.leave};alias()", "E201"),
    ] {
        let mut checker = Checker::new();
        let ast = crate::parser::parse(source).unwrap();
        assert_eq!(
            checker.block(&ast, None, None).unwrap_err().code,
            code,
            "{source}"
        );
        assert!(checker.scope_exits.is_empty());
        for (id, point) in checker.points.iter().enumerate() {
            if point.kind == PointKind::Stmt && !point.complete {
                assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
            }
        }
    }
}

#[test]
pub(crate) fn control_alias_endpoints_reject_inactive_missing_and_foreign_targets() {
    let (mut checker, ast) = opened("'out{finish:'out.leave}");
    let (id, _) = checker.checked_stmt(&ast.stmts[0]).unwrap();
    let target = checker.frames.last().unwrap().id;
    let span = ast.span;
    let count = checker.endpoint_edges;
    for identity in [
        BindingIdentity::Control {
            target: usize::MAX,
            owner: 0,
        },
        BindingIdentity::Control { target, owner: 1 },
    ] {
        assert!(
            checker
                .identity_binding_endpoint(id, identity, span)
                .is_err()
        );
    }
    let identity = BindingIdentity::Control { target, owner: 0 };
    checker.frames.last_mut().unwrap().owner = 1;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    let mut frame = checker.frames.pop().unwrap();
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    frame.owner = 0;
    checker.frames.push(frame);
    checker
        .identity_binding_endpoint(id, identity, span)
        .unwrap();
    assert_eq!(checker.endpoint_edges, count);
}

#[test]
pub(crate) fn control_alias_endpoints_charge_frame_lookup_and_bound_publication() {
    let (mut checker, _) = opened("'out{}");
    let target = checker.frames.last().unwrap().id;
    let ast = crate::parser::parse("finish:'out.leave").unwrap();
    checker.block_start(&ast, None, None, false).unwrap();
    let (id, _) = checker.checked_stmt(&ast.stmts[0]).unwrap();
    assert_ne!(checker.points[id].block, Some(target));
    let identity = BindingIdentity::Control { target, owner: 0 };
    let key = SequenceSource::Stmt(id);
    let count = checker.endpoint_edges;
    checker
        .identity_binding_endpoint(id, identity, ast.span)
        .unwrap();
    assert_eq!(checker.endpoint_edges, count);
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    let total: usize = checker.edge_counts().iter().map(|(_, size)| size).sum();
    checker.sequence_edges += MAX_EDGES - total;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, ast.span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    checker.sequence_edges -= 1;
    let before = checker.flow.work;
    checker
        .identity_binding_endpoint(id, identity, ast.span)
        .unwrap();
    let work = checker.flow.work - before;
    assert!(work > checker.frames.len());
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, ast.span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!(checker.endpoint_edges, count - 1);
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .identity_binding_endpoint(id, identity, ast.span)
        .unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
    assert_eq!(checker.endpoint_edges, count);
}
