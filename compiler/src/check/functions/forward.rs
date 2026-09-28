use super::*;

#[test]
pub(crate) fn forward_groups_retain_checked_sites_and_restore_statement_context() {
    use crate::check::dependencies::SequenceSource;
    let ast = crate::parser::parse("{f<()->int32>;f<int32>:(){->1}}").unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let hir::Stmt::Expr(hir::Expr {
        kind: hir::ExprKind::Block(inner),
        ..
    }) = &body.stmts[0]
    else {
        panic!("block")
    };
    let outer = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    let group = checker.sequences[&SequenceSource::Block(inner.id)].items[0].unwrap();
    let site = &checker.sites[&checker.points[group].site.unwrap()];
    assert_eq!(site.point, Some(group));
    assert_eq!(site.parent, checker.points[outer].site);
    assert_eq!(site.block, Some(inner.id));
    assert!(site.complete);
    assert!(checker.site.is_none());
    assert!(checker.statement.is_empty());
    for capacity in [false, true] {
        let mut checker = Checker::new();
        let ast = crate::parser::parse("f<()->int32>;f<int32>:(){->1}").unwrap();
        if capacity {
            checker.statements = 65_536;
        } else {
            checker.flow.work = crate::flow::MAX_PROOF_WORK;
        }
        assert_eq!(
            checker.forward_point(&ast.stmts, 0).unwrap_err().code,
            "B001"
        );
        assert!(checker.sites.is_empty());
        assert!(checker.points.is_empty());
        assert!(checker.statement.is_empty());
    }
}

#[test]
pub(crate) fn forward_groups_return_reserved_ids_across_reordered_and_nested_definitions() {
    let ast = crate::parser::parse("before<int32>:(){->0};f<()->int32>;g<()->int32>;g<int32>:(){inner<int32>:(){->2};->inner()};f<int32>:(){->g()};tail:3").unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    checker.stmt(&ast.stmts[0]).unwrap();
    let group = checker.forward(&ast.stmts, 1).unwrap();
    assert_eq!(group.end, 5);
    assert_eq!(group.functions.len(), 2);
    for (name, id) in ["f", "g"].into_iter().zip(&group.functions) {
        assert!(
            matches!(checker.value(name, ast.span).unwrap(), Value::Function { id: found, .. } if found == *id)
        );
        let function = checker.functions[*id].as_ref().unwrap();
        assert_eq!(function.id, *id);
        assert_eq!(function.name, name);
    }
    let nested = checker
        .functions
        .iter()
        .flatten()
        .find(|function| function.name == "inner")
        .unwrap();
    assert!(!group.functions.contains(&nested.id));
    assert_eq!(checker.functions.len(), 4);
    assert_eq!(checker.owner, 0);
}

#[test]
pub(crate) fn forward_groups_keep_existing_errors_and_checked_sequence_anchors() {
    for (source, code) in [
        ("f<()->int32>", "E221"),
        ("f<()->int32>;x:1;f<int32>:(){->1}", "E221"),
        ("f<()->int32>;f<boolean>:(){->true}", "E221"),
        ("f<()->int32>;f<int32>:(){->false}", "E207"),
        ("f<()->int32>;f<()->int32>;f<int32>:(){->1}", "E203"),
        ("x:1;f<()->int32>;f<int32>:(){->x}", "E221"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    let ast = crate::parser::parse("f<()->int32>;f<int32>:(){->1};x:2").unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let sequence = &checker.sequences[&crate::check::dependencies::SequenceSource::Block(body.id)];
    assert_eq!(sequence.items.len(), 2);
    let point = sequence.items[0].unwrap();
    assert_eq!(checker.points[point].span, ast.stmts[0].span);
    assert!(checker.points[point].complete);
}

#[test]
pub(crate) fn forward_groups_bound_retained_id_count_and_work_before_growth() {
    let span = Span::new(10, 20);
    let mut group = Forward {
        end: 3,
        functions: vec![0; crate::flow::MAX_NODES - 1],
        site: None,
    };
    let mut flow = crate::flow::Flow::new();
    group.record(7, &mut flow, span).unwrap();
    let error = group.record(8, &mut flow, span).unwrap_err();
    assert_eq!(error.code, "B001");
    assert_eq!(error.span, span);
    assert_eq!(group.functions.len(), crate::flow::MAX_NODES);
    assert_eq!(group.functions.last(), Some(&7));
    let mut group = Forward {
        end: 3,
        functions: Vec::new(),
        site: None,
    };
    let mut flow = crate::flow::Flow::new();
    flow.work = crate::flow::MAX_PROOF_WORK - 1;
    group.record(7, &mut flow, span).unwrap();
    assert_eq!(flow.work, crate::flow::MAX_PROOF_WORK);
    assert!(group.record(8, &mut flow, span).is_err());
    assert_eq!(group.functions, [7]);
    assert_eq!(group.end, 3);
}
