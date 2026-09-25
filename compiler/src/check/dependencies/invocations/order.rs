use super::{tests::check, *};

#[test]
pub(crate) fn call_edges_order_side_effectful_arguments_before_an_opaque_call() {
    let source = "f<int32>:(a<int32>,b<int32>){->a+b};x:=0;y:f({x=1;->x},{x=2;->x})";
    crate::compile(source).unwrap();
    let checker = check(source);
    let call = checker.invocations.values().next().unwrap();
    let [first, second] = call.args[..] else {
        panic!()
    };
    assert_eq!(
        call.edges,
        [
            Edge::new(Port::Entry(call.point), Port::Entry(first), Route::Next),
            Edge::new(Port::Normal(first), Port::Entry(second), Route::Next),
            Edge::new(
                Port::Normal(second),
                Port::Operation(call.point),
                Route::Next
            ),
            Edge::new(
                Port::Operation(call.point),
                Port::Normal(call.point),
                Route::Returned
            ),
        ]
    );
    assert_eq!(
        &source[checker.points[first].span.start..checker.points[first].span.end],
        "{x=1;->x}"
    );
    assert_eq!(
        &source[checker.points[second].span.start..checker.points[second].span.end],
        "{x=2;->x}"
    );
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| op.kind == super::super::OperationKind::Write)
            .count(),
        2
    );
}

#[test]
pub(crate) fn call_edges_keep_empty_and_nested_calls_as_separate_effects() {
    let source = "one<int32>:(){->1};two<int32>:(){->2};sum<int32>:(a<int32>,b<int32>){->a+b};sum(one(),two())";
    crate::compile(source).unwrap();
    let checker = check(source);
    assert_eq!(checker.invocations.len(), 3);
    let outer = checker
        .invocations
        .values()
        .find(|call| call.function == 2)
        .unwrap();
    for (inner, arg) in checker
        .invocations
        .values()
        .filter(|call| call.function != 2)
        .zip(&outer.args)
    {
        assert_eq!(checker.points[inner.point].parent, Some(*arg));
        assert_eq!(
            inner.edges,
            [
                Edge::new(
                    Port::Entry(inner.point),
                    Port::Operation(inner.point),
                    Route::Next
                ),
                Edge::new(
                    Port::Operation(inner.point),
                    Port::Normal(inner.point),
                    Route::Returned
                ),
            ]
        );
    }
}

#[test]
pub(crate) fn call_edges_do_not_turn_never_calls_or_exiting_arguments_into_normal_completion() {
    let source =
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};f:(a<int32>,b<int32>){};f(stop(),1)";
    crate::compile(source).unwrap();
    let checker = check(source);
    let stop = checker
        .invocations
        .values()
        .find(|call| call.function == 0)
        .unwrap();
    let outer = checker
        .invocations
        .values()
        .find(|call| call.function == 1)
        .unwrap();
    assert!(!stop.may_return);
    assert!(
        !stop
            .edges
            .iter()
            .any(|edge| edge.to == Port::Normal(stop.point))
    );
    assert_eq!(checker.points[stop.point].parent, Some(outer.args[0]));
    assert!(outer.edges.contains(&Edge::new(
        Port::Normal(outer.args[0]),
        Port::Entry(outer.args[1]),
        Route::Next
    )));
    for args in ["{'out.leave();->1},2", "1,{'out.leave();->2}"] {
        let source = format!("f:(a<int32>,b<int32>){{}};'out{{f({args})}}");
        crate::compile(&source).unwrap();
        let checker = check(&source);
        assert_eq!(checker.scope_exits.len(), 1);
        let call = checker.invocations.values().next().unwrap();
        assert_eq!(
            call.edges
                .iter()
                .filter(|edge| edge.to == Port::Operation(call.point))
                .count(),
            1
        );
        assert!(call.edges.contains(&Edge::new(
            Port::Normal(call.args[1]),
            Port::Operation(call.point),
            Route::Next
        )));
    }
    assert_eq!(
        crate::compile("f:(a<&!int32>,b<&int32>){};x:=1;f(&!x,&x)").unwrap_err()[0].code,
        "E302"
    );
}

#[test]
pub(crate) fn call_edges_share_the_graph_budget_and_publish_atomically() {
    let mut checker = check("f:(){};f()");
    let call = checker.invocations.values().next().unwrap().clone();
    let count = checker.invocation_edges;
    checker.invocation(call.clone()).unwrap();
    assert_eq!(checker.invocation_edges, count);
    checker.invocations.clear();
    checker.invocation_edges = 0;
    checker.sequence_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .invocation(call)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.invocations.is_empty());
    assert_eq!(checker.invocation_edges, 0);
}
