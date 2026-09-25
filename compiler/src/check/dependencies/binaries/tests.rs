use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let body = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, body)
}

#[test]
pub(crate) fn binary_stages_order_both_primary_projections_without_sequence_bypasses() {
    let source = "a:{->1;->tag:true};b:{->2;->tag:false};x:a+b";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    assert_eq!(op.plan.primary, [true, true]);
    let a = Port::Projection { point: id, step: 0 };
    let b = Port::Projection { point: id, step: 1 };
    assert_eq!(
        op.edges,
        [
            Edge::new(Port::Entry(id), Port::Entry(op.inputs[0]), Route::Next),
            Edge::new(Port::Normal(op.inputs[0]), a, Route::Next),
            Edge::new(Port::Normal(op.inputs[1]), b, Route::Next),
            Edge::new(b, Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Checked),
        ]
    );
    assert_eq!(
        checker.sequences[&SequenceSource::Expr(id)].edges,
        [Edge::new(a, Port::Entry(op.inputs[1]), Route::Next)]
    );
    assert!(!checker.region_edges.contains_key(&id));
    let (checker, _) = check("a:{->n:1};b:{->n:2};x:a==b");
    assert_eq!(
        checker.binaries.values().next().unwrap().plan.primary,
        [false, false]
    );
}

#[test]
pub(crate) fn binary_stages_keep_checked_arithmetic_plain_results_and_call_returns() {
    for symbol in ["+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">="] {
        let source = format!("a:7;b:2;x:a{symbol}b");
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let op = checker.binaries.values().next().unwrap();
        let route = if matches!(symbol, "+" | "-" | "*" | "/" | "%") {
            Route::Checked
        } else {
            Route::Next
        };
        assert_eq!(op.op, symbol);
        assert_eq!(op.edges.last().unwrap().route, route);
    }
    for source in ["x:7.0/2.0", "x:\"a\"<\"b\"", "m:@\"bits\";x:m.and(7,2)"] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        assert_eq!(
            checker
                .binaries
                .values()
                .next()
                .unwrap()
                .edges
                .last()
                .unwrap()
                .route,
            Route::Next
        );
    }
    let (checker, _) = check("f<int32>:(){->1};x:f()+f()");
    let op = checker.binaries.values().next().unwrap();
    assert_eq!(checker.invocations.len(), 2);
    for input in op.inputs {
        let call = checker
            .invocations
            .values()
            .find(|call| call.point == input)
            .unwrap();
        assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    }
}

#[test]
pub(crate) fn binary_stages_keep_stops_short_circuit_required_and_owner_boundaries() {
    for (tail, normal) in [("stop()+1", [false, true]), ("1+stop()", [true, false])] {
        let source = format!("d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};{tail}");
        crate::compile(&source).unwrap();
        let (checker, _) = check(&source);
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        assert_eq!(op.plan.normal, normal);
        assert!(
            !op.edges
                .iter()
                .any(|edge| edge.to == Port::Operation(id) || edge.to == Port::Normal(id))
        );
        assert_eq!(
            checker.sequences[&SequenceSource::Expr(id)].edges.len(),
            usize::from(normal[0])
        );
    }
    let source = "f<never>:(r<{-><never>;tag<boolean>}>){->r+1}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    assert_eq!(op.plan.primary, [true, false]);
    assert!(!op.plan.normal[0]);
    assert!(
        checker.sequences[&SequenceSource::Expr(id)]
            .edges
            .is_empty()
    );
    let (checker, _) = check("false&&(1<2)");
    assert!(
        !checker
            .binaries
            .contains_key(checker.branch_edges.keys().next().unwrap())
    );
    let (checker, _) = check("<T>:{n:1+2;-><uint8[n]>};x<T>:[7]");
    assert!(checker.binaries.is_empty());
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;|flag|x:1+2;f<int32>:(){->3+4}").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.binaries.values().any(|op| op.owner != 0));
    assert!(
        checker
            .binaries
            .values()
            .any(|op| op.owner == 0 && op.control)
    );
}

#[test]
pub(crate) fn binary_stages_preserve_errors_and_atomically_publish_both_ledgers() {
    for (source, code) in [("1/0", "E107"), ("1+false", "E222")] {
        let mut checker = Checker::new();
        assert_eq!(
            checker
                .block(&crate::parser::parse(source).unwrap(), None, None)
                .unwrap_err()
                .code,
            code
        );
        assert!(checker.binaries.is_empty());
    }
    let (mut checker, body) = check("1+2");
    let hir::Stmt::Expr(value) = &body.stmts[0] else {
        panic!()
    };
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    let op = op.clone();
    let counts = (checker.binary_edges, checker.sequence_edges);
    checker
        .binary_operation(id, op.inputs, op.plan, value, op.span)
        .unwrap();
    assert_eq!((checker.binary_edges, checker.sequence_edges), counts);
    checker.binaries.clear();
    checker.binary_edges = 0;
    let seq = checker.sequences.remove(&SequenceSource::Expr(id)).unwrap();
    checker.sequence_edges -= seq.edges.len();
    let mut plan = op.plan;
    plan.primary[0] = true;
    assert!(
        checker
            .binary_operation(id, op.inputs, plan, value, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(
        checker
            .binary_operation(id, [op.inputs[0]; 2], op.plan, value, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.coercion_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .binary_operation(id, op.inputs, op.plan, value, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.binaries.is_empty());
    assert!(!checker.sequences.contains_key(&SequenceSource::Expr(id)));
    assert_eq!((checker.binary_edges, checker.sequence_edges), (0, 0));
}
