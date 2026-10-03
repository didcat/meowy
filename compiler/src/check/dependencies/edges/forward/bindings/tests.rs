use super::*;

pub(super) fn checked(source: &str) -> (Checker, Reports) {
    crate::compile(source).unwrap();
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let program = hir::Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, ast.span).unwrap();
    (checker, reports)
}

#[test]
pub(crate) fn binding_qualification_keeps_contextual_roots_matcher_sites_and_owners() {
    let (mut checker, reports) =
        checked("r<{n<int32>}>:(({->n:1}));f<int32>:(flag<boolean>){|flag|x:1;n:2;->n}");
    let ids = reports
        .effects
        .iter()
        .filter_map(|(&id, (owner, effect))| {
            matches!(
                effect,
                Effect::Storage {
                    kind: OperationKind::Bind,
                    ..
                }
            )
            .then_some((id, *owner))
        })
        .collect::<Vec<_>>();
    assert!(ids.iter().any(|(_, owner)| *owner != 0));
    assert!(
        ids.iter()
            .any(|(id, _)| checker.sites[&checker.points[*id].site.unwrap()].point != Some(*id))
    );
    for (id, owner) in ids {
        let result = checker
            .binding_effect(&reports, id, owner, Span::default())
            .unwrap()
            .unwrap();
        let op = &checker.operations[&id];
        assert_eq!(
            result,
            Binding {
                local: op.local,
                storage: op.storage,
                input: op.input
            }
        );
    }
    assert!(reports.slot_uses.is_empty());
}

#[test]
pub(crate) fn binding_qualification_keeps_stopped_inputs_and_later_body_stops_separate() {
    let (mut checker, reports) = checked("d:@\"debug\";a:1;b:d.panic(\"stop\");c:2");
    let ids = checker
        .operations
        .iter()
        .map(|(&id, op)| (id, op.local))
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    for (index, (id, _)) in ids.into_iter().enumerate() {
        let result = checker
            .binding_effect(&reports, id, 0, Span::default())
            .unwrap();
        assert_eq!(result.is_some(), index == 0);
    }
    assert!(
        reports
            .blocks
            .values()
            .all(|(_, block)| !block.normal && !block.result)
    );
}

#[test]
pub(crate) fn binding_qualification_preserves_synthetic_module_spans() {
    let body = crate::parser::parse("n:1;->value:n").unwrap();
    let span = body.span;
    let ast = crate::ast::Block {
        label: None,
        span: Span::default(),
        stmts: vec![crate::ast::Stmt {
            span,
            kind: crate::ast::StmtKind::Bind {
                name: "\0module1".to_owned(),
                ty: None,
                mutable: false,
                value: crate::ast::Expr {
                    span,
                    kind: crate::ast::ExprKind::Block(body),
                },
            },
        }],
    };
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let block = body.id;
    let program = hir::Program {
        body,
        functions: Vec::new(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, ast.span).unwrap();
    let (&id, op) = checker
        .operations
        .iter()
        .find(|(id, _)| checker.points[**id].block == Some(block))
        .unwrap();
    let input = op.input;
    assert_eq!(checker.bodies[&block].span, Span::default());
    assert_eq!(checker.points[input.unwrap()].span, span);
    assert_eq!(
        checker
            .binding_effect(&reports, id, 0, span)
            .unwrap()
            .unwrap()
            .input,
        input
    );
}
