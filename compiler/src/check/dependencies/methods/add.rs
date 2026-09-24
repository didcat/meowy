use super::{tests::check, *};

#[test]
pub(crate) fn add_captures_receiver_and_length_before_item_effects_and_capacity_check() {
    for (source, length) in [
        ("xs<int32[3]>:=[1];ys:xs.add({xs=[2,3];->4})", None),
        (
            "xs<int32[3]>:=[1];view:&xs;ys:view.add({xs=[2,3];->4})",
            None,
        ),
        ("xs<int32[3]>:[1];ys:xs.add(2)", Some(1)),
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let (&id, method) = checker.methods.first_key_value().unwrap();
        let Kind::Add {
            item,
            capacity,
            length: known,
            may_return,
        } = method.kind
        else {
            panic!()
        };
        assert_eq!(capacity, 3);
        assert_eq!(known, length);
        assert!(may_return);
        assert_ne!(method.receiver, item);
        assert_eq!(
            method.edges,
            [
                Edge::new(Port::Entry(id), Port::Entry(method.receiver), Route::Next),
                Edge::new(
                    Port::Normal(method.receiver),
                    Port::Snapshot(id),
                    Route::Next
                ),
                Edge::new(Port::Snapshot(id), Port::Entry(item), Route::Next),
                Edge::new(Port::Normal(item), Port::Operation(id), Route::Checked),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]
        );
    }
}

#[test]
pub(crate) fn add_preserves_error_order_and_nonreturning_item_boundaries() {
    for (source, code) in [
        ("xs:[1];xs.add(false)", "E207"),
        ("xs:[1];xs.add(2)", "E103"),
        ("xs:[1];xs.add()", "E212"),
        ("xs:=[1];p:&!(xs[1]);ys:xs.add(2);after:*p", "E302"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    for source in [
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs:[1];xs.add(stop())",
        "xs:[1];'out{xs.add({'out.leave()})}",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let (&id, method) = checker.methods.first_key_value().unwrap();
        assert!(matches!(
            method.kind,
            Kind::Add {
                may_return: false,
                ..
            }
        ));
        assert!(
            !method
                .edges
                .iter()
                .any(|edge| edge.to == Port::Operation(id) || edge.to == Port::Normal(id))
        );
    }
}

#[test]
pub(crate) fn add_retains_control_and_function_owner_without_changing_receiver_storage() {
    let source = "flag:false;xs<int32[2]>:[1];|flag|xs.add(2);f:(){ys<int32[2]>:[1];ys.add(2)}";
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert_eq!(checker.methods.len(), 2);
    assert!(checker.methods.values().any(|method| method.control));
    assert!(checker.methods.values().any(|method| method.owner != 0));
    assert!(
        !checker
            .operations
            .values()
            .any(|op| op.kind == super::super::OperationKind::Write)
    );
}

#[test]
pub(crate) fn add_rejects_reused_roots_invalid_lengths_and_exhausted_shared_edges() {
    let mut checker = check("xs<int32[2]>:[1];xs.add(2)");
    let (&id, method) = checker.methods.first_key_value().unwrap();
    let method = method.clone();
    checker.methods.clear();
    checker.method_edges = 0;
    let Kind::Add { item, capacity, .. } = method.kind else {
        panic!()
    };
    for kind in [
        Kind::Add {
            item: method.receiver,
            capacity,
            length: None,
            may_return: true,
        },
        Kind::Add {
            item,
            capacity,
            length: Some(capacity + 1),
            may_return: true,
        },
    ] {
        assert!(
            checker
                .method_operation(id, method.receiver, kind, method.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(checker.methods.is_empty());
    }
    checker.index_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .method_operation(id, method.receiver, method.kind, method.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.methods.is_empty());
    assert_eq!(checker.method_edges, 0);
}
