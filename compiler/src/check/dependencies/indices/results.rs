use super::{tests::check, *};

#[test]
pub(crate) fn index_results_retain_normal_and_stopped_position_decisions() {
    for (source, normal) in [
        ("xs:[1];x:xs[1]", true),
        ("xs:[1];p:&xs;x:p[1]", true),
        ("xs:[1];'out{xs[{'out.leave()}]}", false),
        ("xs:[1];p:&xs;'out{p[{'out.leave()}]}", false),
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let (&id, index) = checker.indices.first_key_value().unwrap();
        let access = index.access.unwrap();
        assert_eq!(access.normal, normal);
        assert_eq!(access.may_return, normal);
        assert_eq!(
            index.edges.iter().any(|edge| edge.to == Port::Normal(id)),
            normal
        );
    }
    let source = "stop<never>:(){'loop{'loop.restart()}};stop()[missing]";
    crate::compile(source).unwrap();
    let checker = check(source);
    assert!(checker.indices.values().next().unwrap().access.is_none());
    for source in ["xs<never[1]>:[]", "f<never>:(xs<never[1]>){->xs[1]}"] {
        let errors = crate::compile(source).unwrap_err();
        assert_eq!(errors[0].code, "B001");
        assert!(errors[0].message.contains("uninhabited list elements"));
    }
}

#[test]
pub(crate) fn index_results_separate_read_and_result_edges_without_relaxing_identity() {
    let mut checker = check("xs:[1];xs[1]");
    let (&id, index) = checker.indices.first_key_value().unwrap();
    let index = index.clone();
    let mut access = index.access.unwrap();
    access.normal = false;
    let count = checker.index_edges;
    assert!(
        checker
            .index_operation(id, index.receiver, index.load, Some(access), index.span)
            .is_err()
    );
    assert_eq!(checker.index_edges, count);
    assert_eq!(checker.indices[&id], index);
    checker.indices.clear();
    checker.index_edges = 0;
    checker
        .index_operation(id, index.receiver, index.load, Some(access), index.span)
        .unwrap();
    let edges = &checker.indices[&id].edges;
    assert!(
        edges
            .iter()
            .any(|edge| edge.to == Port::Operation(id) && edge.route == Route::Checked)
    );
    assert!(!edges.iter().any(|edge| edge.to == Port::Normal(id)));
    checker.indices.clear();
    checker.index_edges = 0;
    access.may_return = false;
    access.normal = true;
    assert!(
        checker
            .index_operation(id, index.receiver, index.load, Some(access), index.span)
            .is_err()
    );
    assert!(checker.indices.is_empty());
    assert_eq!(checker.index_edges, 0);
}
