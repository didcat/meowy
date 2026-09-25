use super::*;
use crate::{ast, check::dependencies::SequenceSource, hir};

pub(crate) fn expr(source: &str) -> ast::Expr {
    let ast::StmtKind::Expr(expr) = crate::parser::parse(source).unwrap().stmts.remove(0).kind
    else {
        panic!()
    };
    expr
}

pub(crate) fn record() -> hir::Type {
    hir::Type::Record {
        primary: Box::new(hir::Type::Null),
        fields: ["x", "y"]
            .into_iter()
            .map(|name| hir::Field {
                name: name.into(),
                ty: hir::Type::Bool,
                mutable: false,
            })
            .collect(),
    }
}

#[test]
pub(crate) fn composed_links_keep_exact_group_roots_and_partial_block_results() {
    let mut checker = Checker::new();
    let expr = expr("(({->x:true}))");
    let mut prior = None;
    for _ in 0..2 {
        let (outer, value) = checker.composed_point(&expr, record(), None).unwrap();
        assert_ne!(prior, Some(outer));
        prior = Some(outer);
        let hir::ExprKind::Block(body) = value.kind else {
            panic!()
        };
        let hir::Type::Record { fields, .. } = &body.ty else {
            panic!()
        };
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "x");
        let mut root = outer;
        for _ in 0..2 {
            let Port::Entry(child) = checker.region_edges[&root][0].to else {
                panic!()
            };
            assert_eq!(
                checker.region_edges[&root][1],
                Edge::new(Port::Normal(child), Port::Normal(root), Route::Next)
            );
            assert_eq!(checker.points[child].parent, Some(root));
            root = child;
        }
        assert_eq!(checker.bodies[&body.id].parent, Some(root));
        assert_eq!(
            checker.endpoints[&SequenceSource::Expr(root)],
            [
                Edge::new(Port::Entry(root), Port::BlockEntry(body.id), Route::Next),
                Edge::new(
                    Port::BlockResult(body.id),
                    Port::Normal(root),
                    Route::Result
                ),
            ]
        );
    }
    assert!(checker.point.is_none());
    crate::compile("v<{x<boolean>;y<boolean>}>:{->(({->x:true}));->y:false}").unwrap();
}

#[test]
pub(crate) fn composed_links_keep_stopped_bodies_owners_and_fallback_boundaries() {
    let source = "d:@\"debug\";f<{x<boolean>}>:(){->(({d.panic(\"stop\")}))}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    let body = &checker.functions[0].as_ref().unwrap().body;
    assert_eq!(body.ty, hir::Type::Never);
    assert!(checker.endpoints.iter().any(|(source, edges)| {
        matches!(source, SequenceSource::Expr(id) if checker.points[*id].owner != 0)
            && edges
                .iter()
                .any(|edge| matches!(edge.to, Port::BlockEntry(_)))
    }));
    assert!(!checker.endpoints.values().flatten().any(|edge| {
        matches!((edge.from, edge.to), (Port::BlockNormal(a), Port::BlockResult(b)) if a == b)
            && matches!(edge.to, Port::BlockResult(id) if id == body.id)
    }));
    let mut checker = Checker::new();
    let (root, value) = checker
        .composed_point(&expr("true"), record(), Some(&hir::Type::Bool))
        .unwrap();
    assert_eq!(value.ty, hir::Type::Bool);
    assert!(!checker.region_edges.contains_key(&root));
    assert!(!checker.endpoints.contains_key(&SequenceSource::Expr(root)));
}

#[test]
pub(crate) fn composed_links_preserve_slot_errors_and_shared_budget_failures() {
    for (source, code) in [
        ("v<{x<boolean>;y<boolean>}>:{->(({->x:true}))}", "E204"),
        ("v<{x<boolean>}>:{->(({->x:1}))}", "E207"),
        ("v<{x<boolean>}>:{->{->x:true};->x:false}", "E205"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    let mut checker = Checker::new();
    checker.dispatch_edges = MAX_EDGES;
    let error = checker
        .composed_point(&expr("(({->x:true}))"), record(), None)
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("budget"));
    assert!(checker.region_edges.is_empty());
    assert!(checker.point.is_none());
}
