use super::{tests::check, *};

#[test]
pub(crate) fn shared_index_loads_follow_receiver_effects_before_position_effects() {
    let source = "d:@\"debug\";get<&int32[3]>:(p<&int32[3]>){d.print(\"get\");->p};xs<int32[3]>:=[1,2];y:get(&xs)[{xs=[];->1}]";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, index) = checker.indices.first_key_value().unwrap();
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(call.point, index.receiver);
    let access = index.access.unwrap();
    let load = Port::Projection { point: id, step: 0 };
    assert!(index.load);
    assert_eq!(access.length, None);
    assert_eq!(checker.invocations.len(), 1);
    assert!(call.edges.contains(&Edge::new(
        Port::Operation(index.receiver),
        Port::Normal(index.receiver),
        Route::Returned
    )));
    assert_eq!(
        index.edges,
        [
            Edge::new(Port::Entry(id), Port::Entry(index.receiver), Route::Next),
            Edge::new(Port::Normal(index.receiver), load, Route::Next),
            Edge::new(load, Port::Snapshot(id), Route::Next),
            Edge::new(
                Port::Snapshot(id),
                Port::Entry(access.position),
                Route::Next
            ),
            Edge::new(
                Port::Normal(access.position),
                Port::Operation(id),
                Route::Checked
            ),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ]
    );
}

#[test]
pub(crate) fn shared_index_loads_preserve_nested_sources_control_and_element_borrows() {
    let source = "flag:false;xs:[[1]];view:&xs;row:{->view:view};|flag|((row.view))[1][1];p:&(view[1]);f<int32>:(v<&int32[1]>){->v[1]}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert_eq!(checker.indices.len(), 3);
    assert_eq!(checker.elements.len(), 1);
    assert_eq!(
        checker.indices.values().filter(|index| index.load).count(),
        2
    );
    for (&id, index) in &checker.indices {
        assert_eq!(index.owner, checker.points[index.receiver].owner);
        assert!(index.control || index.owner != 0);
        assert_eq!(
            index
                .edges
                .iter()
                .any(|edge| edge.to == Port::Projection { point: id, step: 0 }),
            index.load
        );
    }
}

#[test]
pub(crate) fn shared_index_loads_keep_stopped_positions_and_original_diagnostics() {
    let source = "xs:[1];view:&xs;'out{view[{'out.leave()}]}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, index) = checker.indices.first_key_value().unwrap();
    assert!(index.load);
    assert!(!index.access.unwrap().may_return);
    assert_eq!(index.edges.len(), 4);
    assert!(
        !index
            .edges
            .iter()
            .any(|edge| edge.to == Port::Operation(id) || edge.to == Port::Normal(id))
    );
    for (source, code) in [
        ("xs:[1];view:&xs;view[0]", "E101"),
        ("xs:[1];view:&xs;view[false]", "E222"),
        ("xs:=[1];view:&xs;p:&!(xs[1]);view[1];after:*p", "E302"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn shared_index_loads_validate_flags_and_shared_budgets_atomically() {
    let mut checker = check("xs:[1];view:&xs;view[1]");
    let (&id, index) = checker.indices.first_key_value().unwrap();
    let index = index.clone();
    let count = checker.index_edges;
    checker
        .index_operation(id, index.receiver, true, index.access, index.span)
        .unwrap();
    assert_eq!(checker.index_edges, count);
    assert!(
        checker
            .index_operation(id, index.receiver, false, index.access, index.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.indices[&id], index);
    checker.indices.clear();
    checker.index_edges = 0;
    assert!(
        checker
            .index_operation(id, index.receiver, true, None, index.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.method_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .index_operation(id, index.receiver, true, index.access, index.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.indices.is_empty());
    assert_eq!(checker.index_edges, 0);
}
