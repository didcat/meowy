use super::{tests::checked, *};
use crate::{check::dependencies::SequenceSource, hir::Type};

#[test]
pub(crate) fn type_binding_endpoints_cross_resolved_literals_queries_and_meta_values() {
    for source in [
        "kind:(<uint8>);alias:kind;x<(alias)>:1",
        "m:@\"memory\";kind:m.Allocator;alias:kind;x:1",
        "n:1;kind:(n/0)<>;x<(kind)>:2",
        "kind<Type>:{t<Type>:<uint8>;n:2;-><(t)[n]>};alias:kind;x<(alias)>:[1]",
        "c:@\"core\";<Kind>:<c.Type>;t<Kind>:<int32>;x<(t)>:1",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, body) = checked(source);
        let items = checker.sequences[&SequenceSource::Block(body.id)]
            .items
            .clone();
        let graph = checker.forward_index(Span::default()).unwrap();
        let walk = graph
            .walk(
                Port::BlockEntry(body.id),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert!(
            walk.ports
                .contains(&Port::Operation(items.last().unwrap().unwrap())),
            "{source}"
        );
    }
    let (checker, _) = checked("kind<Type>:{t<Type>:<int32>;->t};x:1");
    assert_eq!(
        checker
            .points
            .iter()
            .filter(|point| point.kind == PointKind::Stmt)
            .count(),
        2
    );
    assert_eq!(checker.locals.len(), 1);
    assert_eq!(
        BindingIdentity::capture(&Value::Type(Type::Null)),
        Some(BindingIdentity::Type)
    );
}

#[test]
pub(crate) fn type_binding_endpoints_preserve_query_nonexecution_and_function_owners() {
    let (mut checker, body) = checked("f<int32>:(){->1};kind:f()<>;x:1");
    let graph = checker.forward_index(Span::default()).unwrap();
    let walk = graph
        .walk(
            Port::BlockEntry(body.id),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    for call in checker.invocations.values() {
        assert!(!walk.ports.contains(&Port::Operation(call.point)));
    }
    assert!(!walk.ports.contains(&Port::BlockEntry(
        checker.functions[0].as_ref().unwrap().body.id
    )));
    let (checker, _) = checked("f<int32>:(){kind<Type>:<int32>;->1}");
    let body = &checker.functions[0].as_ref().unwrap().body;
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    assert_eq!(checker.points[id].owner, 1);
    assert_eq!(
        checker.endpoints[&SequenceSource::Stmt(id)],
        [Edge::new(Port::Entry(id), Port::Normal(id), Route::Next)]
    );
    let (checker, body) = checked("<Type>:<uint8>;kind<Type>:3");
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[1].unwrap();
    assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
    assert!(checker.operations.contains_key(&id));
}

#[test]
pub(crate) fn type_binding_endpoints_keep_resolution_and_required_failures_before_publication() {
    for (source, code) in [
        ("kind:=<int32>", "B001"),
        ("kind<Type>:=<int32>", "B001"),
        ("kind<int32>:<int32>", "B001"),
        ("kind<Type>:7", "E207"),
        ("kind<Type>:missing", "E201"),
        ("kind:<int32>;kind:<uint8>", "E203"),
        ("kind<Type>:<int32>;kind<Type>:<uint8>", "E203"),
        ("kind<Type>:{-><int32>;tail:1/0}", "E107"),
        ("d:@\"debug\";kind<Type>:{d.print(1);-><int32>}", "E219"),
        ("n:=2;kind<Type>:{-><int32[n]>}", "E211"),
    ] {
        let mut checker = Checker::new();
        let ast = crate::parser::parse(source).unwrap();
        assert_eq!(
            checker.block(&ast, None, None).unwrap_err().code,
            code,
            "{source}"
        );
        for (id, point) in checker.points.iter().enumerate() {
            if point.kind == PointKind::Stmt && !point.complete {
                assert!(
                    !checker.endpoints.contains_key(&SequenceSource::Stmt(id)),
                    "{source}"
                );
            }
        }
        assert!(checker.type_work.is_none());
    }
    let mut checker = Checker::new();
    let ast = crate::parser::parse("n:3;kind<Type>:{-><int32[n]>}").unwrap();
    checker.stmt(&ast.stmts[0]).unwrap();
    for input in checker.inputs.values_mut() {
        input.derived = true;
    }
    assert_eq!(checker.stmt(&ast.stmts[1]).unwrap_err().code, "E225");
    assert!(checker.endpoints.is_empty());
}

#[test]
pub(crate) fn type_binding_endpoints_preserve_required_costs_and_helper_isolation() {
    let ast = crate::parser::parse("kind<Type>:<int32>").unwrap();
    let root = Span::new(100, 140);
    for remaining in [1, 2, 3] {
        let mut checker = Checker::new();
        let result = checker.required_root(root, |checker| {
            checker.type_work.as_mut().unwrap().logical.types =
                crate::check::required::MAX_TYPES - remaining;
            let result = checker.stmt(&ast.stmts[0]);
            if result.is_ok() {
                assert_eq!(
                    checker.type_work.as_ref().unwrap().logical.types,
                    crate::check::required::MAX_TYPES - remaining + 2
                );
            }
            result
        });
        assert_eq!(result.is_ok(), remaining >= 2);
        if let Err(error) = result {
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, root);
            assert!(checker.endpoints.is_empty());
        }
        assert!(checker.type_work.is_none());
        assert!(checker.locals.is_empty());
    }
    let crate::ast::StmtKind::Bind { value, ty, .. } = &ast.stmts[0].kind else {
        panic!("binding")
    };
    let mut checker = Checker::new();
    assert!(matches!(
        checker.meta_binding(value, ty.as_ref().unwrap()).unwrap(),
        Value::Type(_)
    ));
    assert!(checker.endpoints.is_empty());
    assert!(
        checker
            .points
            .iter()
            .all(|point| point.kind != PointKind::Stmt)
    );
}

#[test]
pub(crate) fn type_binding_endpoints_bound_atomic_publication() {
    let (mut checker, body) = checked("kind<Type>:<int32>");
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    let key = SequenceSource::Stmt(id);
    let span = Span::default();
    let identity = BindingIdentity::Type;
    let count = checker.endpoint_edges;
    checker
        .identity_binding_endpoint(id, identity, span)
        .unwrap();
    assert_eq!(checker.endpoint_edges, count);
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    let total: usize = checker.edge_counts().iter().map(|(_, size)| size).sum();
    checker.sequence_edges += MAX_EDGES - total;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    checker.sequence_edges -= 1;
    let before = checker.flow.work;
    checker
        .identity_binding_endpoint(id, identity, span)
        .unwrap();
    let work = checker.flow.work - before;
    checker.endpoints.remove(&key);
    checker.endpoint_edges -= 1;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .identity_binding_endpoint(id, identity, span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!(checker.endpoint_edges, count - 1);
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .identity_binding_endpoint(id, identity, span)
        .unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
    assert_eq!(checker.endpoint_edges, count);
}
