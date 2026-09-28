use super::*;
use crate::{ast, check::dependencies::SequenceSource};

pub(super) fn initializer(source: &str, name: &str) -> ast::Stmt {
    let body = crate::parser::parse(source).unwrap();
    let span = body.span;
    ast::Stmt {
        span,
        kind: ast::StmtKind::Bind {
            name: name.into(),
            ty: None,
            mutable: false,
            value: ast::Expr {
                span,
                kind: ast::ExprKind::Block(body),
            },
        },
    }
}

pub(super) fn checked(sources: &[&str]) -> (Checker, hir::Block) {
    let ast = ast::Block {
        label: None,
        span: Span::default(),
        stmts: sources
            .iter()
            .enumerate()
            .map(|(index, source)| initializer(source, &format!("\0module{}", index + 1)))
            .collect(),
    };
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    (checker, body)
}

#[test]
pub(crate) fn module_initializers_link_exact_roots_and_order_storage_after_bodies() {
    let (mut checker, body) = checked(&["a:1;->n:a", "->f<int32>:(){->3};->n:2"]);
    let items = checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .clone();
    let program = hir::Program {
        body: body.clone(),
        functions: checker.functions.iter().flatten().cloned().collect(),
        locals: checker.locals.clone(),
    };
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let walk = &reports.entries[&0].1;
    let mut prior = 0;
    for id in items.into_iter().flatten() {
        let op = &checker.operations[&id];
        let input = op.input.unwrap();
        let module = &checker.exports[&op.local];
        assert_eq!(checker.points[input].parent, Some(id));
        assert_eq!(checker.points[input].block, checker.points[id].block);
        assert_eq!(checker.points[input].owner, op.owner);
        assert!(checker.points[input].complete);
        assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(id)));
        for port in [
            Port::Entry(id),
            Port::Entry(input),
            Port::BlockEntry(module.block),
            Port::Normal(input),
            Port::Operation(id),
            Port::Normal(id),
        ] {
            let position = walk.ports.iter().position(|found| *found == port).unwrap();
            assert!(position > prior);
            prior = position;
        }
        assert_eq!(reports.effects[&id].0, 0);
    }
    for function in &program.functions {
        assert!(!walk.ports.contains(&Port::BlockEntry(function.body.id)));
    }
}

#[test]
pub(crate) fn module_initializers_preserve_stopped_sources_and_later_module_boundaries() {
    for first in ["d:@\"debug\";d.panic(\"stop\")", "'again{'again.restart()}"] {
        let (mut checker, body) = checked(&[first, "->n:2"]);
        let items = checker.sequences[&SequenceSource::Block(body.id)]
            .items
            .clone();
        let first = items[0].unwrap();
        let input = checker.operations[&first].input.unwrap();
        let graph = checker.forward_index(Span::default()).unwrap();
        let walk = graph
            .walk(
                Port::BlockEntry(body.id),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert!(walk.ports.contains(&Port::Entry(input)));
        assert!(!walk.ports.contains(&Port::Normal(input)));
        assert!(!walk.ports.contains(&Port::Operation(first)));
        assert!(!walk.ports.contains(&Port::Entry(items[1].unwrap())));
    }
}

#[test]
pub(crate) fn module_initializers_keep_expression_export_and_documentation_errors_first() {
    for (source, code) in [
        ("->n:missing", "E201"),
        ("->n:=1", "B001"),
        ("->f<int32>:(){->false}", "E207"),
    ] {
        let mut checker = Checker::new();
        let stmt = initializer(source, "\0module1");
        assert_eq!(
            checker.checked_stmt(&stmt).unwrap_err().code,
            code,
            "{source}"
        );
        for (id, point) in checker.points.iter().enumerate() {
            if point.kind == PointKind::Stmt && !point.complete {
                assert!(!checker.operations.contains_key(&id));
            }
        }
        assert!(checker.exports.is_empty());
    }
    let source = "#| Export. |#->n:1";
    let parsed = crate::parser::parse_documented(source).unwrap();
    let mut model = crate::documentation::Model::at(source, &parsed, 0, false).unwrap();
    model
        .entries
        .iter_mut()
        .find(|entry| entry.doc.is_some())
        .unwrap()
        .stage = usize::MAX - 1;
    let mut checker = Checker::new();
    let stmt = initializer(source, "\0module1");
    checker.file_docs.insert(stmt.span.start, model);
    let error = checker.checked_stmt(&stmt).unwrap_err();
    assert!(
        error
            .message
            .contains("documentation for an unanalyzed declaration")
    );
    assert!(checker.exports.is_empty());
    assert!(
        !checker
            .operations
            .iter()
            .any(|(id, _)| checker.points[*id].parent.is_none())
    );
}

#[test]
pub(crate) fn module_initializers_preserve_source_identity_and_atomic_three_edge_budgets() {
    let (mut checker, body) = checked(&["->n:1"]);
    let id = checker.sequences[&SequenceSource::Block(body.id)].items[0].unwrap();
    let op = checker.operations.remove(&id).unwrap();
    let input = op.input.unwrap();
    assert_eq!(op.edges.len(), 3);
    checker.operation_edges -= op.edges.len();
    let count = checker.operation_edges;
    let parent = checker.points[input].parent.take();
    assert!(
        checker
            .storage_operation(id, Kind::Bind, op.local, Some(input), op.span)
            .is_err()
    );
    assert!(!checker.operations.contains_key(&id));
    checker.points[input].parent = parent;
    checker.points[input].complete = false;
    assert!(
        checker
            .storage_operation(id, Kind::Bind, op.local, Some(input), op.span)
            .is_err()
    );
    assert!(!checker.operations.contains_key(&id));
    checker.points[input].complete = true;
    let total: usize = checker.edge_counts().iter().map(|(_, size)| size).sum();
    checker.sequence_edges += super::super::edges::MAX_EDGES - total - 2;
    assert!(
        checker
            .storage_operation(id, Kind::Bind, op.local, Some(input), op.span)
            .is_err()
    );
    assert!(!checker.operations.contains_key(&id));
    assert_eq!(checker.operation_edges, count);
    checker.sequence_edges -= 1;
    let before = checker.flow.work;
    checker
        .storage_operation(id, Kind::Bind, op.local, Some(input), op.span)
        .unwrap();
    let work = checker.flow.work - before;
    checker
        .storage_operation(id, Kind::Bind, op.local, Some(input), op.span)
        .unwrap();
    assert_eq!(checker.operation_edges, count + 3);
    checker.operations.remove(&id);
    checker.operation_edges -= 3;
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .storage_operation(id, Kind::Bind, op.local, Some(input), op.span)
            .is_err()
    );
    assert!(!checker.operations.contains_key(&id));
    assert_eq!(checker.operation_edges, count);
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    checker
        .storage_operation(id, Kind::Bind, op.local, Some(input), op.span)
        .unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
    assert_eq!(checker.operation_edges, count + 3);
}
