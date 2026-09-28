use super::{tests::checked, *};
use crate::{check::Value, hir};

#[test]
pub(crate) fn export_endpoints_cross_definitions_and_reexports_without_entering_bodies() {
    let source = "->f<int32>:(){inner<int32>:(){->2};->inner()};x:1;->alias<()->int32>:f;->again<()->int32>:alias;y:2";
    crate::compile(source).unwrap();
    let (mut checker, body) = checked(source);
    let items = checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .clone();
    for index in [0, 2, 3] {
        let id = items[index].unwrap();
        assert_eq!(
            checker.endpoints[&SequenceSource::Stmt(id)],
            [Edge::new(Port::Entry(id), Port::Normal(id), Route::Next)]
        );
        assert_eq!(checker.points[id].owner, 0);
    }
    let graph = checker.forward_index(Span::default()).unwrap();
    let walk = graph
        .walk(
            Port::BlockEntry(body.id),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    assert!(walk.ports.contains(&Port::Operation(items[4].unwrap())));
    for function in checker.functions.iter().flatten() {
        assert!(!walk.ports.contains(&Port::BlockEntry(function.body.id)));
    }
    assert_eq!(checker.functions.len(), 2);
}

#[test]
pub(crate) fn export_endpoints_reuse_imported_ids_with_independent_module_sequences() {
    let block = |source| {
        let body = crate::parser::parse(source).unwrap();
        crate::ast::Expr {
            span: body.span,
            kind: crate::ast::ExprKind::Block(body),
        }
    };
    let mut checker = Checker::new();
    let (source, exports) = checker
        .module_value(&block("->f<int32>:(){->7};x:1"), None)
        .unwrap();
    let Value::Function { id: function, .. } = exports.values["f"] else {
        panic!("function")
    };
    let id = checker.local(source.ty.clone());
    checker.exports.insert(id, exports);
    checker
        .declare(
            "source",
            Value::FileModule {
                id,
                ty: source.ty.clone(),
            },
            source.span,
        )
        .unwrap();
    let (facade, exports) = checker
        .module_value(&block("->alias<()->int32>:source.f;y:2"), None)
        .unwrap();
    assert!(matches!(exports.values["alias"], Value::Function { id, .. } if id == function));
    assert_eq!(checker.functions.len(), 1);
    let graph = checker.forward_index(Span::default()).unwrap();
    for module in [source, facade] {
        let hir::ExprKind::Block(body) = module.kind else {
            panic!("module")
        };
        let items = &checker.sequences[&SequenceSource::Block(body.id)].items;
        let export = items[0].unwrap();
        assert!(
            checker
                .endpoints
                .contains_key(&SequenceSource::Stmt(export))
        );
        assert_eq!(checker.points[export].block, Some(body.id));
        assert_eq!(checker.points[export].owner, 0);
        let walk = graph
            .walk(
                Port::BlockEntry(body.id),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert!(walk.ports.contains(&Port::Operation(items[1].unwrap())));
        assert!(!walk.ports.contains(&Port::BlockEntry(
            checker.functions[function].as_ref().unwrap().body.id
        )));
    }
}

#[test]
pub(crate) fn export_endpoints_keep_never_calls_and_forward_type_barriers() {
    for (tail, present) in [("x:1", true), ("alias();x:1", false)] {
        let source = format!(
            "->stop<never>:(){{'again{{'again.restart()}}}};->alias<()->never>:stop;{tail}"
        );
        crate::compile(&source).unwrap();
        let (mut checker, body) = checked(&source);
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
        assert_eq!(walk.ports.contains(&Port::Operation(last)), present);
    }
    for source in [
        "c:@\"core\";->t<c.Type>:<int32>;x:1",
        "f<()->int32>;f<int32>:(){->1};->alias<()->int32>:f;x:1",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, body) = checked(source);
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
        assert!(!walk.ports.contains(&Port::Operation(last)), "{source}");
        assert!(!walk.missing.is_empty());
    }
}

#[test]
pub(crate) fn export_endpoints_publish_only_after_ordinary_export_checks() {
    for (source, code) in [
        ("->f:(){->1}", "E214"),
        ("->f<int32>:=(){->1}", "B001"),
        ("->f<int32>:(){inner<int32>:(){->1};->false}", "E207"),
        ("->f<int32>:(){->1};->f<int32>:(){->2}", "E205"),
        ("->f:1;->f<int32>:(){->2}", "E205"),
        ("f:1;->f<int32>:(){->2}", "E203"),
        ("f<int32>:(){->1};->alias<()->boolean>:f", "E207"),
        ("|true|->f<int32>:(){->1}", "B001"),
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
    }
}

#[test]
pub(crate) fn export_endpoints_preserve_atomic_identity_and_budget_validation() {
    for source in [
        "->f<int32>:(){->1}",
        "f<int32>:(){->1};->alias<()->int32>:f",
    ] {
        let (mut checker, body) = checked(source);
        let id = checker.sequences[&SequenceSource::Block(body.id)]
            .items
            .last()
            .unwrap()
            .unwrap();
        let key = SequenceSource::Stmt(id);
        let span = Span::default();
        let count = checker.endpoint_edges;
        checker.function_declaration_endpoint(id, 0, span).unwrap();
        assert_eq!(checker.endpoint_edges, count);
        checker.endpoints.remove(&key);
        checker.endpoint_edges -= 1;
        let function = checker.functions[0].take();
        assert!(checker.function_declaration_endpoint(id, 0, span).is_err());
        assert!(!checker.endpoints.contains_key(&key));
        checker.functions[0] = function;
        checker.owner = 1;
        assert!(checker.function_declaration_endpoint(id, 0, span).is_err());
        assert!(!checker.endpoints.contains_key(&key));
        checker.owner = 0;
        let total: usize = checker.edge_counts().iter().map(|(_, size)| size).sum();
        checker.sequence_edges += MAX_EDGES - total;
        assert!(checker.function_declaration_endpoint(id, 0, span).is_err());
        assert!(!checker.endpoints.contains_key(&key));
        assert_eq!(checker.endpoint_edges, count - 1);
        checker.sequence_edges -= 1;
        let before = checker.flow.work;
        checker.function_declaration_endpoint(id, 0, span).unwrap();
        let work = checker.flow.work - before;
        checker.endpoints.remove(&key);
        checker.endpoint_edges -= 1;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
        assert!(checker.function_declaration_endpoint(id, 0, span).is_err());
        assert!(!checker.endpoints.contains_key(&key));
        assert_eq!(checker.endpoint_edges, count - 1);
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
        checker.function_declaration_endpoint(id, 0, span).unwrap();
        assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        assert_eq!(checker.endpoint_edges, count);
    }
}
