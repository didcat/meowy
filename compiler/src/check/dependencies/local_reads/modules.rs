use super::*;
use crate::{ast, check::Value};

pub(crate) fn expr(source: &str) -> ast::Expr {
    let ast::StmtKind::Expr(expr) = crate::parser::parse(source).unwrap().stmts.remove(0).kind
    else {
        panic!()
    };
    expr
}

pub(crate) fn register(checker: &mut Checker, name: &str, source: &str) -> usize {
    let parsed = crate::parser::parse_documented(source).unwrap();
    let span = parsed.block.span;
    let (value, exports) = checker
        .module_value(
            &ast::Expr {
                kind: ast::ExprKind::Block(parsed.block),
                span,
            },
            None,
        )
        .unwrap();
    let id = checker.local(value.ty.clone());
    checker.exports.insert(id, exports);
    checker
        .declare(name, Value::FileModule { id, ty: value.ty }, span)
        .unwrap();
    id
}

#[test]
pub(crate) fn file_module_reads_keep_direct_import_and_alias_storage_identity() {
    let mut checker = Checker::new();
    let local = register(&mut checker, "m", "->3;->n:7");
    checker
        .declare(
            "internal",
            Value::Local {
                id: local,
                ty: checker.locals[local].clone(),
                mutable: false,
                owner: 0,
                constant: None,
            },
            Span::default(),
        )
        .unwrap();
    let stmt = crate::parser::parse("alias:m").unwrap().stmts.remove(0);
    checker.stmt(&stmt).unwrap();
    assert!(checker.local_reads.is_empty());
    for source in ["m", "alias", "@\"./value.mwy\""] {
        let expr = expr(source);
        if source.starts_with('@') {
            checker.imports.insert(expr.span.start, "internal".into());
        }
        let (id, value) = checker.expr_point(&expr, None).unwrap();
        let read = &checker.local_reads[&id];
        assert_eq!(read.local, local);
        assert_eq!(read.storage, local);
        assert_eq!(read.owner, 0);
        assert!(read.normal);
        assert_eq!(read.span, expr.span);
        assert!(matches!(value.kind, hir::ExprKind::Local(found) if found == local));
        assert_eq!(
            read.edges,
            [
                Edge::new(Port::Entry(id), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next)
            ]
        );
        assert!(!checker.narrowings.contains_key(&id));
    }
}

#[test]
pub(crate) fn file_module_reads_precede_field_and_primary_consumers() {
    let mut checker = Checker::new();
    let local = register(&mut checker, "m", "->3;->n:7");
    let (outer, _) = checker.expr_point(&expr("m.n"), None).unwrap();
    let raw = checker.narrowings[&outer].input;
    let field = &checker.fields[&raw];
    assert_eq!(checker.local_reads[&field.input].local, local);
    let int = hir::Type::Int {
        bits: 32,
        signed: true,
    };
    let (outer, value) = checker.expr_point(&expr("m"), Some(&int)).unwrap();
    let coerce = &checker.coercions[&outer];
    assert!(coerce.primary);
    assert_eq!(checker.local_reads[&coerce.input].storage, local);
    assert_eq!(value.ty, int);
    assert!(!checker.narrowings.contains_key(&coerce.input));
}

#[test]
pub(crate) fn file_module_reads_preserve_required_values_static_exports_and_capture_errors() {
    let mut checker = Checker::new();
    register(
        &mut checker,
        "m",
        "->3;->n:7;->revision:@\"proof\".revision",
    );
    register(&mut checker, "s", "->3");
    for source in [
        "<T>:{-><uint8[s]>}",
        "<U>:{-><uint8[m.n]>}",
        "<V>:{-><uint8[m.revision]>}",
    ] {
        checker
            .stmt(&crate::parser::parse(source).unwrap().stmts.remove(0))
            .unwrap();
        assert!(checker.local_reads.is_empty(), "{source}");
    }
    let error = checker
        .stmt(&crate::parser::parse("f:(){m}").unwrap().stmts.remove(0))
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("file-module values"));
    assert!(checker.local_reads.is_empty());
    let mut checker = Checker::new();
    register(&mut checker, "m", "->n:1");
    assert_eq!(
        checker
            .expr_point(&expr("m.private"), None)
            .unwrap_err()
            .code,
        "E201"
    );
    assert!(checker.local_reads.is_empty());
}

#[test]
pub(crate) fn file_module_reads_keep_control_and_atomic_shared_budget_failures() {
    let mut checker = Checker::new();
    register(&mut checker, "m", "->3");
    checker.control = true;
    let (id, _) = checker.expr_point(&expr("m"), None).unwrap();
    assert!(checker.local_reads[&id].control);
    let mut checker = Checker::new();
    register(&mut checker, "m", "->3");
    checker.narrowing_edges = super::super::edges::MAX_EDGES;
    let error = checker.expr_point(&expr("m"), None).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("local-read budget"));
    assert!(checker.local_reads.is_empty());
    assert_eq!(checker.local_read_edges, 0);
    assert!(checker.point.is_none());
}
