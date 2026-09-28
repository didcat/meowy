use super::{tests::checked, *};
use crate::{check::dependencies::SequenceSource, hir};

#[test]
pub(crate) fn meta_export_endpoints_cross_exports_without_creating_runtime_fields() {
    let source = "->kind<Type>:<uint8>;->copy<Type>:kind;->items<Type>:{n:2;-><(copy)[n]>};x:1";
    crate::compile(source).unwrap();
    let (mut checker, body) = checked(source);
    assert_eq!(body.stmts.len(), 1);
    assert_eq!(checker.locals.len(), 1);
    assert_eq!(checker.module.values.len(), 3);
    let Value::Type(kind) = &checker.module.values["kind"] else {
        panic!("type")
    };
    let Value::Type(copy) = &checker.module.values["copy"] else {
        panic!("type")
    };
    assert_eq!(kind, copy);
    let items = checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .clone();
    for id in items[..3].iter().flatten() {
        assert_eq!(checker.points[*id].owner, 0);
        assert_eq!(
            checker.endpoints[&SequenceSource::Stmt(*id)],
            [Edge::new(Port::Entry(*id), Port::Normal(*id), Route::Next)]
        );
    }
    let graph = checker.forward_index(Span::default()).unwrap();
    let walk = graph
        .walk(
            Port::BlockEntry(body.id),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    assert!(walk.ports.contains(&Port::Operation(items[3].unwrap())));
}

#[test]
pub(crate) fn meta_export_endpoints_preserve_imported_payloads_and_module_owners() {
    let block = |source| {
        let body = crate::parser::parse(source).unwrap();
        crate::ast::Expr {
            span: body.span,
            kind: crate::ast::ExprKind::Block(body),
        }
    };
    let mut checker = Checker::new();
    let (source, exports) = checker
        .module_value(&block("-><Kind>:<Type>;->element<Type>:<uint8>"), None)
        .unwrap();
    let Value::Type(element) = &exports.values["element"] else {
        panic!("type")
    };
    let element = element.clone();
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
    let (facade, exports) = checker.module_value(&block("kind:source.element;copy<source.Kind>:kind;->items<source.Kind>:{-><(copy)[2]>};x:1"), None).unwrap();
    assert!(
        matches!(&exports.values["items"], Value::Type(hir::Type::List { element: found, capacity: 2 }) if **found == element)
    );
    let hir::ExprKind::Block(body) = facade.kind else {
        panic!("module")
    };
    let items = checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .clone();
    for id in items[..3].iter().flatten() {
        assert_eq!(checker.points[*id].owner, 0);
        assert_eq!(checker.points[*id].block, Some(body.id));
        assert!(checker.endpoints.contains_key(&SequenceSource::Stmt(*id)));
    }
    let graph = checker.forward_index(Span::default()).unwrap();
    let walk = graph
        .walk(
            Port::BlockEntry(body.id),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    assert!(walk.ports.contains(&Port::Operation(items[3].unwrap())));
}

#[test]
pub(crate) fn meta_export_endpoints_keep_all_export_errors_before_publication() {
    for (source, code) in [
        ("->kind<Type>:=<int32>", "B001"),
        ("|true|->kind<Type>:<int32>", "B001"),
        ("row:{->kind<Type>:<int32>}", "B001"),
        ("->kind<Type>:7", "E207"),
        ("->kind<Type>:missing", "E201"),
        ("->kind<Type>:<Type>", "B001"),
        ("->kind<Type>:<int32>;->kind<Type>:<uint8>", "E205"),
        ("->kind:7;->kind<Type>:{-><uint8>;tail:1/0}", "E205"),
        ("kind:7;->kind<Type>:<int32>", "E203"),
        ("->kind<Type>:{-><int32>;tail:1/0}", "E107"),
        ("p:@\"proof\";->kind<Type>:p.can_copy<uint8>()", "E223"),
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
}

#[test]
pub(crate) fn meta_export_endpoints_keep_logical_and_graph_exhaustion_separate() {
    let ast = crate::parser::parse("->kind<Type>:<int32>").unwrap();
    let root = Span::new(100, 140);
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    let error = checker
        .required_root(root, |checker| {
            checker.type_work.as_mut().unwrap().logical.types =
                crate::check::required::MAX_TYPES - 1;
            checker.stmt(&ast.stmts[0])
        })
        .unwrap_err();
    assert_eq!(error.code, "E220");
    assert_eq!(error.span, root);
    assert!(checker.module.values.is_empty());
    assert!(checker.endpoints.is_empty());
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    checker.sequence_edges = MAX_EDGES;
    let error = checker.stmt(&ast.stmts[0]).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(
        error
            .message
            .starts_with("proof identity binding endpoint budget exhausted")
    );
    assert!(matches!(checker.module.values["kind"], Value::Type(_)));
    assert!(checker.endpoints.is_empty());
    assert_eq!(checker.endpoint_edges, 0);
}
