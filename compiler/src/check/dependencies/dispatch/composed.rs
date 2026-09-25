use super::{tests::check, *};

#[test]
pub(crate) fn composed_dispatch_keeps_partial_fields_and_grouped_receiver_identities() {
    let record = hir::Type::Record {
        primary: Box::new(hir::Type::Null),
        fields: ["x", "y"]
            .into_iter()
            .map(|name| hir::Field {
                name: name.into(),
                ty: hir::Type::Int {
                    signed: true,
                    bits: 32,
                },
                mutable: false,
            })
            .collect(),
    };
    let stmt = crate::parser::parse("((3.{->x:$}))")
        .unwrap()
        .stmts
        .remove(0);
    let crate::ast::StmtKind::Expr(expr) = stmt.kind else {
        panic!()
    };
    let mut checker = Checker::new();
    let (outer, result) = checker.composed_point(&expr, record, None).unwrap();
    let hir::ExprKind::Block(body) = result.kind else {
        panic!()
    };
    let hir::Type::Record { fields, .. } = &body.ty else {
        panic!()
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "x");
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    assert_eq!(op.block, body.id);
    let hir::Stmt::Bind { id: local, .. } = body.stmts[0] else {
        panic!()
    };
    assert_eq!(op.local, local);
    assert_eq!(checker.points[op.input].parent, Some(id));
    let group = checker.points[id].parent.unwrap();
    assert_eq!(checker.region_edges[&group][0].to, Port::Entry(id));
    assert_eq!(checker.region_edges[&outer][0].to, Port::Entry(group));
    assert!(!checker.endpoints.contains_key(&SequenceSource::Expr(id)));
    assert_eq!(
        op.edges.last().unwrap(),
        &Edge::new(Port::BlockResult(body.id), Port::Normal(id), Route::Result)
    );
    crate::compile("v<{x<int32>;y<int32>}>:{->((3.{->x:$}));->y:4}").unwrap();
}

#[test]
pub(crate) fn composed_dispatch_preserves_call_order_empty_and_opaque_body_boundaries() {
    let source = "g<int32>:(){->3};f<{x<int32>}>:(){->g().{->x:$}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let op = checker.dispatch_ops.values().next().unwrap();
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(call.point, op.input);
    assert_ne!(op.owner, 0);
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    for (source, empty) in [
        ("v<{x<int32>}>:{->3.{};->x:4}", true),
        (
            "f<{x<int32>}>:(){->3.{g<()->int32>;g<int32>:(){->1};->x:$}}",
            false,
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        assert_eq!(
            op.edges.iter().any(|edge| edge.from == Port::Operation(id)),
            empty
        );
        if empty {
            assert!(op.edges.contains(&Edge::new(
                Port::Operation(id),
                Port::BlockNormal(op.block),
                Route::Next
            )));
        }
    }
}

#[test]
pub(crate) fn composed_dispatch_preserves_stops_and_caller_specific_error_order() {
    for (tail, edges) in [("stop().{}", 2), ("3.{stop()}", 4)] {
        let source = format!(
            "d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};f<{{x<int32>}}>:(){{->{tail}}}"
        );
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        assert_eq!(op.edges.len(), edges);
        assert!(!op.edges.iter().any(|edge| edge.to == Port::Normal(id)));
    }
    for (source, code) in [
        ("v<{x<int32>;y<int32>}>:{->3.{->x:$}}", "E204"),
        ("v<{x<int32>}>:{->3.{->x:true}}", "E207"),
        ("v<{x<int32>}>:{->3.{$=4;->x:$}}", "E305"),
        ("n:=1;v<{x<int32>}>:{->(&!n).{->x:missing}}", "E201"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
