use super::*;
use crate::{ast, check::dependencies::SequenceSource, hir};

pub(super) fn registered(source: &str) -> (Checker, hir::LocalId, hir::BlockId) {
    let mut checker = Checker::new();
    let body = crate::parser::parse(source).unwrap();
    let span = body.span;
    let (value, exports) = checker
        .module_value(
            &ast::Expr {
                kind: ast::ExprKind::Block(body),
                span,
            },
            None,
        )
        .unwrap();
    let block = exports.block;
    let id = checker.local(value.ty.clone());
    checker.exports.insert(id, exports);
    checker
        .declare(
            "internal",
            Value::Local {
                id,
                ty: value.ty.clone(),
                mutable: false,
                owner: 0,
                constant: None,
            },
            span,
        )
        .unwrap();
    checker
        .declare("m", Value::FileModule { id, ty: value.ty }, span)
        .unwrap();
    (checker, id, block)
}

#[test]
pub(crate) fn module_alias_endpoints_keep_import_and_alias_ids_without_reentering_initializers() {
    let (mut checker, module, init) = registered("private:99;->n:7");
    let locals = checker.locals.len();
    let ast =
        crate::parser::parse("direct:@\"./value.mwy\";alias:(direct);copy:alias;x:copy.n").unwrap();
    let ast::StmtKind::Bind { value, .. } = &ast.stmts[0].kind else {
        panic!("import binding")
    };
    checker.imports.insert(value.span.start, "internal".into());
    assert_eq!(
        BindingIdentity::capture(&checker.import_module("internal", value.span).unwrap()),
        Some(BindingIdentity::FileModule(module))
    );
    let body = checker.block(&ast, None, None).unwrap();
    let items = checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .clone();
    assert_eq!(body.stmts.len(), 1);
    assert_eq!(checker.locals.len(), locals + 1);
    assert_eq!(checker.exports.len(), 1);
    for id in items[..3].iter().flatten() {
        assert_eq!(
            checker.endpoints[&SequenceSource::Stmt(*id)],
            [Edge::new(Port::Entry(*id), Port::Normal(*id), Route::Next)]
        );
        assert!(!checker.operations.contains_key(id));
    }
    assert!(
        checker
            .local_reads
            .values()
            .any(|read| read.local == module)
    );
    let graph = checker.forward_index(Span::default()).unwrap();
    let walk = graph
        .walk(
            Port::BlockEntry(body.id),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    assert!(walk.ports.contains(&Port::Operation(items[3].unwrap())));
    assert!(!walk.ports.contains(&Port::BlockEntry(init)));
}

#[test]
pub(crate) fn module_alias_endpoints_preserve_function_owners_and_required_only_reads() {
    let (mut checker, module, _) = registered("->2");
    let ast = crate::parser::parse(
        "f<int32>:(){alias:m;copy:alias;<Items>:{-><int32[copy]>};xs<Items>:[9];->xs[1]}",
    )
    .unwrap();
    checker.block(&ast, None, None).unwrap();
    let body = &checker.functions[0].as_ref().unwrap().body;
    let owner = checker.bodies[&body.id].owner;
    for id in checker.sequences[&SequenceSource::Block(body.id)].items[..2]
        .iter()
        .flatten()
    {
        assert_eq!(checker.points[*id].owner, owner);
        assert!(checker.endpoints.contains_key(&SequenceSource::Stmt(*id)));
    }
    assert_ne!(owner, 0);
    assert!(
        !checker
            .local_reads
            .values()
            .any(|read| read.local == module)
    );
    let (mut checker, _, _) = registered("->2");
    let ast = crate::parser::parse("<T>:{alias:m;-><uint8[alias]>}").unwrap();
    let points = checker.points.len();
    checker.block(&ast, None, None).unwrap();
    assert_eq!(
        checker.points[points..]
            .iter()
            .filter(|point| point.kind == PointKind::Stmt)
            .count(),
        1
    );
    assert!(checker.type_work.is_none());
}

#[test]
pub(crate) fn module_alias_endpoints_preserve_privacy_binding_and_capture_errors() {
    for (source, code) in [
        ("alias:m;alias:m", "E203"),
        ("alias:=m", "B001"),
        ("alias<int32>:m", "B001"),
        ("alias:m;x:alias.private", "E201"),
        ("alias:m;<T>:<alias.Private>", "E202"),
        ("f:(){alias:m;alias.n}", "B001"),
    ] {
        let (mut checker, _, _) = registered("private:9;<Private>:<int32>;->n:7");
        let ast = crate::parser::parse(source).unwrap();
        assert_eq!(
            checker.block(&ast, None, None).unwrap_err().code,
            code,
            "{source}"
        );
        for (id, point) in checker.points.iter().enumerate() {
            if point.kind == PointKind::Stmt && !point.complete {
                assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
            }
        }
    }
}

#[test]
pub(crate) fn module_alias_endpoints_reject_missing_local_or_exports_registration() {
    let (mut checker, module, _) = registered("->2");
    let ast = crate::parser::parse("alias:m").unwrap();
    let (id, _) = checker.checked_stmt(&ast.stmts[0]).unwrap();
    let span = ast.span;
    let count = checker.endpoint_edges;
    assert!(
        checker
            .identity_binding_endpoint(id, BindingIdentity::FileModule(usize::MAX), span)
            .is_err()
    );
    let exports = checker.exports.remove(&module).unwrap();
    assert!(
        checker
            .identity_binding_endpoint(id, BindingIdentity::FileModule(module), span)
            .is_err()
    );
    checker.exports.insert(module, exports);
    let locals = std::mem::take(&mut checker.locals);
    assert!(
        checker
            .identity_binding_endpoint(id, BindingIdentity::FileModule(module), span)
            .is_err()
    );
    checker.locals = locals;
    checker
        .identity_binding_endpoint(id, BindingIdentity::FileModule(module), span)
        .unwrap();
    assert_eq!(checker.endpoint_edges, count);
}

#[test]
pub(crate) fn module_alias_endpoints_bound_registry_lookup_and_atomic_publication() {
    let (mut checker, module, _) = registered("->2");
    let ast = crate::parser::parse("alias:m").unwrap();
    let (id, _) = checker.checked_stmt(&ast.stmts[0]).unwrap();
    let identity = BindingIdentity::FileModule(module);
    let key = SequenceSource::Stmt(id);
    let count = checker.endpoint_edges;
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
    checker
        .identity_binding_endpoint(id, identity, ast.span)
        .unwrap();
    assert_eq!(checker.endpoint_edges, count);
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

#[test]
pub(crate) fn module_alias_endpoints_follow_captured_synthetic_initialization_sources() {
    let mut ast = crate::parser::parse("seed:{->n:7};m:@\"./value.mwy\";copy:m;x:1").unwrap();
    let ast::StmtKind::Bind { name, .. } = &mut ast.stmts[0].kind else {
        panic!("initializer")
    };
    *name = "\0module1".into();
    let ast::StmtKind::Bind { value, .. } = &ast.stmts[1].kind else {
        panic!("import")
    };
    let mut checker = Checker::new();
    checker.imports.insert(value.span.start, "\0module1".into());
    let body = checker.block(&ast, None, None).unwrap();
    let items = checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .clone();
    let init = items[0].unwrap();
    let input = checker.operations[&init].input.unwrap();
    assert_eq!(checker.points[input].parent, Some(init));
    assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(init)));
    assert!(
        checker
            .endpoints
            .contains_key(&SequenceSource::Stmt(items[1].unwrap()))
    );
    let graph = checker.forward_index(Span::default()).unwrap();
    let walk = graph
        .walk(
            Port::BlockEntry(body.id),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    assert!(!walk.missing.contains(&Port::Entry(init)));
    assert!(walk.ports.contains(&Port::Operation(items[3].unwrap())));
}
