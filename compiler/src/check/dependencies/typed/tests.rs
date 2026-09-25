use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn typed_stages_keep_kinds_noop_sources_and_effect_before_result_order() {
    for (source, kind) in [
        ("v:7~<int32>", Kind::Ascription),
        ("v:7<boolean>", Kind::Predicate),
        ("v:7<never>", Kind::Predicate),
        ("f<int32>:(){->7};v:f()~<int32>", Kind::Ascription),
        ("f<int32>:(){->7};v:f()<int32>", Kind::Predicate),
        ("r:{->n:1};v:r.n~<int32>", Kind::Ascription),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, op) = checker.typed_ops.first_key_value().unwrap();
        assert_eq!(op.kind, kind);
        assert!(op.normal);
        assert_eq!(checker.points[op.input].parent, Some(id));
        assert!(!checker.region_edges.contains_key(&id));
        assert_eq!(
            op.edges,
            [
                Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next),
                Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]
        );
        for call in checker.invocations.values() {
            assert_eq!(call.point, op.input);
            assert_eq!(call.edges.last().unwrap().route, Route::Returned);
        }
    }
    let (checker, _) = check("v:((false&&true)<boolean>)~<boolean>");
    assert_eq!(checker.typed_ops.len(), 2);
    assert_eq!(checker.branch_edges.len(), 1);
}

#[test]
pub(crate) fn typed_stages_keep_compile_time_targets_separate_and_preserve_control() {
    let source = "n:2;v:[1,2]~<({-><int32[n]>})>";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let op = checker.typed_ops.values().next().unwrap();
    let reads = checker.body_inputs.values().flatten().collect::<Vec<_>>();
    assert!(!reads.is_empty());
    for read in reads {
        assert_ne!(read.point, op.input);
        assert!(
            !op.edges
                .iter()
                .any(|edge| edge.to == Port::Entry(read.point))
        );
    }
    let source = "flag:false;v<int32><null>:1;|flag|x:v<int32>;f<null>:(v<int32><null>){|v<int32>|x:v~<int32>}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert!(checker.typed_ops.values().any(|op| op.owner != 0));
    assert!(
        checker
            .typed_ops
            .values()
            .any(|op| op.owner == 0 && op.control)
    );
    let (checker, _) = check("p:@\"proof\";q:p.can_copy<uint8>();<F>:q.always<>;<T>:7<>");
    assert!(checker.typed_ops.is_empty());
}

#[test]
pub(crate) fn typed_stages_preserve_stopped_operands_and_original_rejections() {
    for tail in ["stop()<int32>", "stop()~<int32>"] {
        let source = format!("d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};v:{tail}");
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let (&id, op) = checker.typed_ops.first_key_value().unwrap();
        assert!(!op.normal);
        assert_eq!(
            op.edges,
            [Edge::new(
                Port::Entry(id),
                Port::Entry(op.input),
                Route::Next
            )]
        );
        let call = checker.invocations.values().next().unwrap();
        assert_eq!(call.point, op.input);
        assert!(!call.may_return);
    }
    for (source, code) in [
        ("v:7~<boolean>", "E208"),
        ("v:(1/0)~<Missing>", "E107"),
        ("d:@\"debug\";v:d.panic(\"stop\")<Missing>", "E202"),
        ("p:@\"proof\";q:p.can_copy<uint8>();v:q.always", "B001"),
    ] {
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .block(&crate::parser::parse(source).unwrap(), None, None)
                .unwrap_err()
                .code,
            code
        );
        assert!(checker.typed_ops.is_empty());
        assert!(checker.point.is_none());
    }
}

#[test]
pub(crate) fn typed_stages_publish_atomically_with_exact_identity_and_shared_budgets() {
    let (mut checker, block) = check("v:7~<int32>");
    let hir::Stmt::Bind { value, .. } = &block.stmts[0] else {
        panic!()
    };
    let (&id, op) = checker.typed_ops.first_key_value().unwrap();
    let op = op.clone();
    let count = checker.typed_edges;
    checker
        .typed_operation(id, op.input, false, value, op.span)
        .unwrap();
    assert_eq!(checker.typed_edges, count);
    assert!(
        checker
            .typed_operation(id, op.input, true, value, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.typed_ops.clear();
    checker.typed_edges = 0;
    for case in 0..3 {
        match case {
            0 => checker.points[op.input].parent = None,
            1 => checker.points[op.input].complete = false,
            2 => checker.points[op.input].owner += 1,
            _ => unreachable!(),
        }
        assert!(
            checker
                .typed_operation(id, op.input, false, value, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        checker.points[op.input].parent = Some(id);
        checker.points[op.input].complete = true;
        checker.points[op.input].owner = op.owner;
    }
    checker.field_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .typed_operation(id, op.input, false, value, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.typed_ops.is_empty());
    assert_eq!(checker.typed_edges, 0);
}
