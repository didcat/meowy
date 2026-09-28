use super::{tests::checked, *};
use crate::{
    check::{Constant, dependencies::SequenceSource},
    hir::Type,
};

#[test]
pub(crate) fn static_binding_endpoints_cross_aliases_and_preserve_uint32_payloads() {
    let source = "p:@\"proof\";module:p;revision:module.revision;copy:(revision);<T>:{n:copy;-><uint8[n]>};x<T>:[7]";
    crate::compile(source).unwrap();
    let (mut checker, body) = checked(source);
    let items = checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .clone();
    for id in items[..5].iter().flatten() {
        assert_eq!(
            checker.endpoints[&SequenceSource::Stmt(*id)],
            [Edge::new(Port::Entry(*id), Port::Normal(*id), Route::Next)]
        );
    }
    assert_eq!(checker.locals.len(), 1);
    let graph = checker.forward_index(Span::default()).unwrap();
    let walk = graph
        .walk(
            Port::BlockEntry(body.id),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    assert!(walk.ports.contains(&Port::Operation(items[5].unwrap())));
    let ast = crate::parser::parse("p:@\"proof\";revision:p.revision;copy:revision").unwrap();
    let mut checker = Checker::new();
    for stmt in &ast.stmts {
        assert!(checker.stmt(stmt).unwrap().is_empty());
    }
    for name in ["revision", "copy"] {
        assert!(matches!(
            checker.value(name, ast.span).unwrap(),
            Value::Static {
                value: Constant::Int(1),
                ty: Type::Int {
                    bits: 32,
                    signed: false
                }
            }
        ));
    }
    assert!(checker.locals.is_empty());
    assert_eq!(
        BindingIdentity::capture(&Value::Static {
            value: Constant::Bool(true),
            ty: Type::Bool
        }),
        Some(BindingIdentity::Static)
    );
    assert_eq!(
        BindingIdentity::capture(&Value::Constant(Constant::Int(1))),
        None
    );
}

#[test]
pub(crate) fn static_binding_endpoints_preserve_typed_mutable_and_shadowed_runtime_storage() {
    let source = "p:@\"proof\";revision:p.revision;typed<uint32>:revision;changed:=revision;changed=2;copy:changed";
    crate::compile(source).unwrap();
    let (checker, body) = checked(source);
    let items = &checker.sequences[&SequenceSource::Block(body.id)].items;
    for index in [2, 3, 5] {
        let id = items[index].unwrap();
        assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
        assert!(checker.operations.contains_key(&id));
    }
    assert!(checker.locals.iter().all(|ty| *ty
        == Type::Int {
            bits: 32,
            signed: false
        }));
    let source = "p:{->revision:7};revision:p.revision;copy:revision";
    crate::compile(source).unwrap();
    let (checker, body) = checked(source);
    for id in checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .iter()
        .flatten()
    {
        assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(*id)));
        assert!(checker.operations.contains_key(id));
    }
    let (checker, _) = checked("f<uint32>:(){p:@\"proof\";revision:p.revision;->revision}");
    let body = &checker.functions[0].as_ref().unwrap().body;
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[1].unwrap();
    assert_eq!(checker.points[id].owner, 1);
    assert!(checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
}

#[test]
pub(crate) fn static_binding_endpoints_keep_required_only_aliases_and_charges_separate() {
    let ast = crate::parser::parse("<T>:{n:p.revision;copy:n;-><uint8[copy]>}").unwrap();
    let crate::ast::StmtKind::TypeAlias { ty, .. } = &ast.stmts[0].kind else {
        panic!("type alias")
    };
    let root = Span::new(100, 150);
    let mut costs = Vec::new();
    for ordinary in [false, true] {
        let mut checker = Checker::new();
        checker
            .declare("p", Value::Module(Module::Proof), root)
            .unwrap();
        costs.push(
            checker
                .mode_root(root, true, |checker| {
                    if ordinary {
                        checker.stmt(&ast.stmts[0])?;
                    } else {
                        checker.declare_type("T", ty, false, ast.span)?;
                    }
                    let budget = &checker.type_work.as_ref().unwrap().logical;
                    Ok((budget.steps, budget.types, budget.slots))
                })
                .unwrap(),
        );
        assert_eq!(
            checker
                .points
                .iter()
                .filter(|point| point.kind == PointKind::Stmt)
                .count(),
            usize::from(ordinary)
        );
        assert_eq!(checker.endpoint_edges, usize::from(ordinary));
        assert!(checker.locals.is_empty());
        assert!(checker.type_work.is_none());
    }
    assert_eq!(costs[0], costs[1]);
    assert!(costs[0].0 > 0 && costs[0].1 > 0);
}

#[test]
pub(crate) fn static_binding_endpoints_preserve_errors_and_stopped_boundaries() {
    for (source, code) in [
        ("<T>:{p:@\"proof\";n:p.revision;-><uint8[n]>}", "B001"),
        (
            "p:@\"proof\";revision:p.revision;revision:p.revision",
            "E203",
        ),
        ("p:@\"proof\";revision:p.revision;v<int32>:revision", "E207"),
        (
            "p:@\"proof\";revision:p.revision;v:revision+4294967296",
            "E216",
        ),
        (
            "p:@\"proof\";revision:=p.revision;<T>:{n:revision;-><uint8[n]>}",
            "E211",
        ),
        (
            "p:@\"proof\";revision:p.revision;<T>:{n:revision/0;-><uint8[n]>}",
            "E107",
        ),
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
    for source in [
        "stop<never>:(){'loop{'loop.restart()}};stop();p:@\"proof\";revision:p.revision;x:1",
        "f<()->int32>;f<int32>:(){->1};p:@\"proof\";revision:p.revision;x:1",
        "'out{finish:'out.leave};p:@\"proof\";revision:p.revision;x:1",
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
    }
    assert_eq!(
        crate::compile("p:@\"proof\";revision:p.revision;r:p.can_copy<uint32>()").unwrap_err()[0]
            .code,
        "B001"
    );
}

#[test]
pub(crate) fn static_binding_endpoints_bound_atomic_publication_after_declaration() {
    let ast = crate::parser::parse("p:@\"proof\";revision:p.revision").unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    checker.stmt(&ast.stmts[0]).unwrap();
    let count = checker.endpoint_edges;
    let total: usize = checker.edge_counts().iter().map(|(_, size)| size).sum();
    checker.sequence_edges += MAX_EDGES - total;
    let error = checker.stmt(&ast.stmts[1]).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(
        error
            .message
            .starts_with("proof identity binding endpoint budget exhausted")
    );
    assert_eq!(checker.endpoint_edges, count);
    assert!(matches!(
        checker.value("revision", ast.span).unwrap(),
        Value::Static { .. }
    ));
    let id = checker
        .points
        .iter()
        .position(|point| point.kind == PointKind::Stmt && !point.complete)
        .unwrap();
    assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
    checker.sequence_edges -= 1;
    checker.point = Some(id);
    let before = checker.flow.work;
    checker
        .identity_binding_endpoint(id, BindingIdentity::Static, ast.span)
        .unwrap();
    let work = checker.flow.work - before;
    checker
        .identity_binding_endpoint(id, BindingIdentity::Static, ast.span)
        .unwrap();
    assert_eq!(checker.endpoint_edges, count + 1);
    checker.endpoints.remove(&SequenceSource::Stmt(id));
    checker.endpoint_edges -= 1;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .identity_binding_endpoint(id, BindingIdentity::Static, ast.span)
            .is_err()
    );
    assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .identity_binding_endpoint(id, BindingIdentity::Static, ast.span)
        .unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
    assert_eq!(checker.endpoint_edges, count + 1);
}
