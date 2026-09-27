use super::{tests::check, *};

#[test]
pub(crate) fn shared_method_loads_follow_call_returns_before_size_or_add() {
    for (method, next) in [("size()", false), ("add({xs=[];->2})", true)] {
        let source = format!(
            "d:@\"debug\";get<&int32[3]>:(p<&int32[3]>){{d.print(\"get\");->p}};xs<int32[3]>:=[1];get(&xs).{method}"
        );
        crate::compile(&source).unwrap();
        let checker = check(&source);
        let (&id, method) = checker.methods.first_key_value().unwrap();
        let call = checker.invocations.values().next().unwrap();
        assert_eq!(checker.invocations.len(), 1);
        assert_eq!(call.point, method.receiver);
        assert!(method.load);
        assert!(call.edges.contains(&Edge::new(
            Port::Operation(call.point),
            Port::Normal(call.point),
            Route::Returned
        )));
        let load = Port::Projection { point: id, step: 0 };
        let target = if next {
            Port::Snapshot(id)
        } else {
            Port::Operation(id)
        };
        assert_eq!(
            method.edges[1],
            Edge::new(Port::Normal(method.receiver), load, Route::Next)
        );
        assert_eq!(method.edges[2], Edge::new(load, target, Route::Next));
        assert!(!method.edges.contains(&Edge::new(
            Port::Normal(method.receiver),
            target,
            Route::Next
        )));
        if let Kind::Add { item, length, .. } = method.kind {
            assert_eq!(length, None);
            assert_eq!(
                method.edges[3],
                Edge::new(Port::Snapshot(id), Port::Entry(item), Route::Next)
            );
        }
    }
}

#[test]
pub(crate) fn shared_method_loads_retain_grouped_fields_reborrows_and_owners() {
    let source = "flag:false;xs<int32[2]>:[1];view:&xs;row:{->view:view};|flag|((row.view)).size();(&*view).add(2);f<uint64>:(v<&int32[2]>){->v.size()}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert_eq!(checker.methods.len(), 3);
    assert!(checker.methods.values().all(|method| method.load));
    assert!(checker.methods.values().any(|method| method.control));
    assert!(checker.methods.values().any(|method| method.owner != 0));
    for method in checker.methods.values() {
        assert_eq!(method.owner, checker.points[method.receiver].owner);
    }
}

#[test]
pub(crate) fn shared_method_loads_preserve_stopped_items_and_rejections() {
    let source = "xs:[1];view:&xs;'out{view.add({'out.leave()})}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, method) = checker.methods.first_key_value().unwrap();
    assert!(method.load);
    assert!(matches!(
        method.kind,
        Kind::Add {
            may_return: false,
            ..
        }
    ));
    assert_eq!(method.edges.len(), 4);
    assert!(
        !method
            .edges
            .iter()
            .any(|edge| edge.to == Port::Operation(id) || edge.to == Port::Normal(id))
    );
    let source = "(&[1]).add(2)";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, method) = checker.methods.first_key_value().unwrap();
    let Kind::Add {
        item,
        capacity,
        length,
        ..
    } = method.kind
    else {
        panic!()
    };
    assert!(method.load);
    assert_eq!((capacity, length), (1, None));
    assert!(method.edges.contains(&Edge::new(
        Port::Normal(item),
        Port::Operation(id),
        Route::Checked
    )));
    for (source, code) in [
        ("xs:[1];view:&xs;view.size(missing)", "E212"),
        ("xs:[1];view:&xs;view.add()", "E212"),
        ("xs:[1];view:&xs;view.add(false)", "E207"),
        ("xs:[1];xs.add(2)", "E103"),
        ("xs:=[1];view:&xs;p:&!(xs[1]);view.size();after:*p", "E302"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn shared_method_loads_validate_flags_and_shared_budgets_atomically() {
    let mut checker = check("xs:[1];view:&xs;view.size()");
    let (&id, method) = checker.methods.first_key_value().unwrap();
    let method = method.clone();
    let count = checker.method_edges;
    checker
        .method_operation(id, method.receiver, true, method.kind, method.span)
        .unwrap();
    assert_eq!(checker.method_edges, count);
    assert!(
        checker
            .method_operation(id, method.receiver, false, method.kind, method.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.methods[&id], method);
    checker.methods.clear();
    checker.method_edges = 0;
    for kind in [Kind::StringSize, Kind::Stopped] {
        assert!(
            checker
                .method_operation(id, method.receiver, true, kind, method.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    checker.index_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .method_operation(id, method.receiver, true, method.kind, method.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.methods.is_empty());
    assert_eq!(checker.method_edges, 0);
}
