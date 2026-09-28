use super::{tests::checked, *};
use crate::{ast, check::dependencies::SequenceSource};

#[test]
pub(crate) fn record_aliases_keep_runtime_storage_and_initializer_inputs() {
    let source = "row:{->part:{->n:3;->flag:true}};copy:row;part:copy.part;again:part";
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    let mut locals = Vec::new();
    for stmt in &ast.stmts {
        let ast::StmtKind::Bind { name, value, .. } = &stmt.kind else {
            panic!("binding")
        };
        let (point, _) = checker.checked_stmt(stmt).unwrap();
        let binding = checker.value(name, stmt.span).unwrap();
        let Value::Local { id, owner, .. } = &binding else {
            panic!("runtime local")
        };
        assert_eq!(BindingIdentity::capture(&binding), None);
        assert!(!checker.endpoints.contains_key(&SequenceSource::Stmt(point)));
        let operation = &checker.operations[&point];
        assert_eq!(operation.local, *id);
        assert_eq!(operation.storage, *id);
        assert_eq!(operation.owner, *owner);
        let input = operation.input.unwrap();
        assert_eq!(checker.points[input].parent, Some(point));
        assert_eq!(checker.points[input].span, value.span);
        assert!(operation.edges.contains(&Edge::new(
            Port::Normal(input),
            Port::Operation(point),
            Route::Next
        )));
        assert!(checker.record_inputs.contains_key(id));
        assert!(!locals.contains(id));
        locals.push(*id);
    }
    assert_eq!(locals.len(), 4);
}

#[test]
pub(crate) fn record_aliases_in_required_scopes_keep_only_original_reads_and_outer_completion() {
    let prefix = "row:{->part:{->n:3;->flag:true}};runtime:row;";
    let tail = "<T>:{copy:runtime;part:copy.part;alias:part;|alias.flag|-><uint8[alias.n]>;|!alias.flag|-><uint8[1]>};xs<T>:[7]";
    for function in [false, true] {
        let source = if function {
            format!("f<uint8>:(){{{prefix}{tail};->xs[1]}}")
        } else {
            format!("{prefix}{tail}")
        };
        crate::compile(&source).unwrap();
        let (mut checker, main) = checked(&source);
        let body = if function {
            checker.functions[0].as_ref().unwrap().body.clone()
        } else {
            main
        };
        let items = checker.sequences[&SequenceSource::Block(body.id)]
            .items
            .clone();
        let point = items[2].unwrap();
        let runtime = checker.operations[&items[1].unwrap()].local;
        let reads = &checker.body_inputs[&body.id];
        assert_eq!(reads.len(), 1);
        let read = &reads[0];
        assert_eq!(read.id, runtime);
        assert_eq!(read.storage, runtime);
        assert_eq!(&source[read.span.start..read.span.end], "runtime");
        assert!(source[read.root.start..read.root.end].starts_with("{copy:runtime;"));
        assert_eq!(checker.points[read.point].owner, usize::from(function));
        assert!(checker.points[read.point].complete);
        assert_eq!(
            checker.endpoints[&SequenceSource::Stmt(point)],
            [Edge::new(
                Port::Entry(point),
                Port::Normal(point),
                Route::Next
            )]
        );
        assert!(
            !checker
                .points
                .iter()
                .any(|point| point.kind == PointKind::Stmt
                    && point.span.start >= read.root.start
                    && point.span.end <= read.root.end)
        );
        assert!(checker.scopes.iter().all(|scope| {
            !scope
                .values
                .values()
                .any(|value| matches!(value, Value::Record { .. }))
        }));
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
}

#[test]
pub(crate) fn record_aliases_do_not_turn_stopped_paths_into_completion() {
    let source = "stop<never>:(){'loop{'loop.restart()}};row:{->n:3};stop();copy:row;<T>:{scratch:copy;-><uint8[scratch.n]>};xs<T>:[7]";
    crate::compile(source).unwrap();
    let (mut checker, body) = checked(source);
    let items = checker.sequences[&SequenceSource::Block(body.id)]
        .items
        .clone();
    let graph = checker.forward_index(Span::default()).unwrap();
    let walk = graph
        .walk(
            Port::BlockEntry(body.id),
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    for point in items[3..].iter().flatten() {
        assert!(checker.points[*point].complete);
        assert!(!walk.ports.contains(&Port::Entry(*point)));
        assert!(!walk.ports.contains(&Port::Normal(*point)));
    }
}

#[test]
pub(crate) fn record_aliases_keep_required_failures_before_outer_publication() {
    for (source, code) in [
        ("row:{->n:=3};<T>:{copy:row;-><uint8[copy.n]>}", "E211"),
        (
            "d:@\"debug\";row:{->n:3;d.print(9)};<T>:{copy:row;-><uint8[copy.n]>}",
            "E211",
        ),
        ("row:{->n:3};<T>:{copy:row;copy:row;-><uint8>}", "E203"),
        ("row:{->n:3};<T>:{copy:row;-><uint8>};leaked:copy", "E201"),
        (
            "seed:{->n<uint8>:255};row:{->part:{->n:3};->bad:seed.n+1};<T>:{part:row.part;-><uint8[part.n]>}",
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
        for (point, item) in checker
            .points
            .iter()
            .enumerate()
            .filter(|(_, item)| item.kind == PointKind::Stmt && !item.complete)
        {
            assert!(
                !checker.endpoints.contains_key(&SequenceSource::Stmt(point)),
                "{source}: {item:?}"
            );
        }
        assert!(checker.type_work.is_none());
        assert!(checker.scopes.iter().all(|scope| {
            !scope
                .values
                .values()
                .any(|value| matches!(value, Value::Record { .. }))
        }));
    }
}

#[test]
pub(crate) fn record_aliases_share_required_costs_and_fail_before_completion_at_the_limit() {
    let source = "row:{->part:{->n:3}};<T>:{copy:row;part:copy.part;alias:part;-><uint8[alias.n]>}";
    let ast = crate::parser::parse(source).unwrap();
    let ast::StmtKind::TypeAlias { ty, .. } = &ast.stmts[1].kind else {
        panic!("type alias")
    };
    let mut costs = Vec::new();
    for ordinary in [false, true] {
        let mut checker = Checker::new();
        checker.block_start(&ast, None, None, false).unwrap();
        checker.stmt(&ast.stmts[0]).unwrap();
        let locals = checker.locals.len();
        costs.push(
            checker
                .mode_root(ty.span, true, |checker| {
                    if ordinary {
                        checker.stmt(&ast.stmts[1])?;
                    } else {
                        checker.declare_type("T", ty, false, ast.stmts[1].span)?;
                    }
                    let work = &checker.type_work.as_ref().unwrap().logical;
                    Ok((work.steps, work.types, work.slots))
                })
                .unwrap(),
        );
        assert_eq!(checker.locals.len(), locals);
        assert!(checker.type_work.is_none());
    }
    assert_eq!(costs[0], costs[1]);
    assert!(costs[0].0 > 0 && costs[0].1 > 0 && costs[0].2 > 0);
    for spare in [0, 1] {
        let mut checker = Checker::new();
        checker.block_start(&ast, None, None, false).unwrap();
        checker.stmt(&ast.stmts[0]).unwrap();
        let endpoints = checker.endpoints.clone();
        let result = checker.mode_root(ty.span, true, |checker| {
            checker.type_work.as_mut().unwrap().logical.steps =
                crate::check::required::MAX_STEPS - costs[0].0 + spare;
            checker.checked_stmt(&ast.stmts[1])
        });
        if spare == 0 {
            let (point, _) = result.unwrap();
            assert!(checker.endpoints.contains_key(&SequenceSource::Stmt(point)));
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.code, "E220");
            assert_eq!(error.span, ty.span);
            assert_eq!(checker.endpoints, endpoints);
        }
        assert!(checker.type_work.is_none());
    }
}

#[test]
pub(crate) fn record_aliases_do_not_launder_derived_inputs_into_required_values() {
    let source = "row:{->part:{->n:3}};<T>:{copy:row;part:copy.part;-><uint8[part.n]>}";
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    checker.block_start(&ast, None, None, false).unwrap();
    checker.stmt(&ast.stmts[0]).unwrap();
    for input in checker.record_inputs.values_mut() {
        input.input.derived = true;
    }
    let endpoints = checker.endpoints.clone();
    let error = checker.checked_stmt(&ast.stmts[1]).unwrap_err();
    assert_eq!(error.code, "E225");
    assert_eq!(&source[error.span.start..error.span.end], "row");
    assert!(checker.body_inputs.is_empty());
    assert_eq!(checker.endpoints, endpoints);
    assert!(checker.type_work.is_none());
}
