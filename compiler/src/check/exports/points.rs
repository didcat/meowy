use super::*;
use crate::check::dependencies::PointKind;

pub(super) fn block(source: &str) -> ast::Expr {
    let body = crate::parser::parse(source).unwrap();
    ast::Expr {
        span: body.span,
        kind: ExprKind::Block(body),
    }
}

#[test]
pub(crate) fn module_value_points_return_exact_roots_after_nested_checking() {
    let source = block("->f<int32>:(){inner<int32>:(){->2};->inner()};->n:7");
    let mut checker = Checker::new();
    let (stmt, (point, value, module)) = checker
        .with_point_id(PointKind::Stmt, source.span, |checker| {
            checker.module_value_point(&source, None)
        })
        .unwrap();
    assert_ne!(point, stmt);
    assert_eq!(checker.points[point].parent, Some(stmt));
    assert_eq!(checker.points[point].kind, PointKind::Expr);
    assert_eq!(checker.points[point].owner, 0);
    assert!(checker.points[point].complete);
    let hir::ExprKind::Block(body) = value.kind else {
        panic!("module body")
    };
    assert_eq!(module.block, body.id);
    assert!(module.values.contains_key("f"));
    assert!(module.inputs.contains_key("n"));
    assert_eq!(checker.functions.len(), 2);
    assert!(
        checker
            .functions
            .iter()
            .flatten()
            .all(|function| checker.bodies[&function.body.id].owner != checker.points[point].owner)
    );
    assert!(checker.point.is_none());
}

#[test]
pub(crate) fn module_value_points_keep_expected_outer_roots_and_never_results() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let mut checker = Checker::new();
    let source = block("private:1;->private");
    let (point, value, _) = checker.module_value_point(&source, Some(&int)).unwrap();
    assert_eq!(value.ty, int);
    let raw = checker.coercions[&point].input;
    assert_ne!(raw, point);
    assert_eq!(checker.points[raw].parent, Some(point));
    let mut checker = Checker::new();
    let (point, value, module) = checker
        .module_value_point(&block("d:@\"debug\";d.panic(\"stop\")"), None)
        .unwrap();
    assert_eq!(value.ty, Type::Never);
    assert!(checker.points[point].complete);
    assert_eq!(
        checker.proofs.completions[&module.block],
        crate::flow::FALSE
    );
}

#[test]
pub(crate) fn module_value_points_restore_context_before_expression_and_documentation_failures() {
    let mut checker = Checker::new();
    checker.module.block = 17;
    checker.module.depth = 23;
    checker
        .module
        .values
        .insert("outer".into(), Value::Type(Type::Bool));
    let outer = crate::parser::parse_documented("").unwrap();
    checker.documentation = Some(crate::documentation::Model::new("", &outer).unwrap());
    let work = checker.documentation.as_ref().unwrap().work;
    let invalid = ast::Expr {
        kind: ExprKind::Int("1".into()),
        span: Span::default(),
    };
    assert_eq!(
        checker
            .module_value_point(&invalid, None)
            .err()
            .unwrap()
            .code,
        "B001"
    );
    assert!(checker.points.is_empty());
    for docs in [false, true] {
        let source = if docs {
            "#| Export. |#->n:1"
        } else {
            "->n:missing"
        };
        let parsed = crate::parser::parse_documented(source).unwrap();
        let value = ast::Expr {
            span: parsed.block.span,
            kind: ExprKind::Block(parsed.block.clone()),
        };
        if docs {
            let mut model = crate::documentation::Model::at(source, &parsed, 0, false).unwrap();
            let entry = model
                .entries
                .iter_mut()
                .find(|entry| entry.doc.is_some())
                .unwrap();
            entry.stage = usize::MAX - 1;
            checker.file_docs.insert(value.span.start, model);
        }
        let error = checker.module_value_point(&value, None).err().unwrap();
        assert_eq!(error.code, if docs { "B001" } else { "E201" });
        if docs {
            assert!(
                error
                    .message
                    .contains("documentation for an unanalyzed declaration")
            );
        }
        assert_eq!((checker.module.block, checker.module.depth), (17, 23));
        assert!(checker.module.values.contains_key("outer"));
        assert_eq!(checker.documentation.as_ref().unwrap().work, work);
        assert!(checker.point.is_none());
    }
}
