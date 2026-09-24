use super::{tests::check, *};

#[test]
pub(crate) fn indexed_stores_capture_each_index_and_length_before_rhs() {
    let source = "r:={->rows:=[{->xs:=[1,2]}]};i:=1;r.rows[{i=1;->i}].xs[{i=2;->i}]={i=1;->3}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, op) = checker.paths.first_key_value().unwrap();
    assert_eq!(checker.paths.len(), 1);
    assert_eq!(op.steps.len(), 4);
    assert_eq!(op.steps[0], Step::Field(0));
    assert_eq!(op.steps[2], Step::Field(0));
    let mut roots = Vec::new();
    for (step, capacity, text) in [(1, 1, "{i=1;->i}"), (3, 2, "{i=2;->i}")] {
        let Step::Index {
            point,
            capacity: size,
            span,
        } = op.steps[step]
        else {
            panic!()
        };
        assert_eq!(size, capacity);
        assert_eq!(checker.points[point].parent, Some(id));
        assert_eq!(
            &source[checker.points[point].span.start..checker.points[point].span.end],
            text
        );
        assert!(span.start < span.end);
        let reserve = Port::Reserve { point: id, step };
        assert!(op.edges.contains(&Edge::new(
            Port::Address { point: id, step },
            reserve,
            Route::Next
        )));
        assert!(
            op.edges
                .contains(&Edge::new(reserve, Port::Entry(point), Route::Next))
        );
        assert!(op.edges.contains(&Edge::new(
            Port::Normal(point),
            Port::Address {
                point: id,
                step: step + 1
            },
            Route::Checked
        )));
        assert!(
            !op.edges
                .iter()
                .any(|edge| edge.from == Port::Normal(point) && edge.route == Route::Next)
        );
        roots.push(point);
    }
    assert_ne!(roots[0], roots[1]);
    assert!(!roots.contains(&op.input));
    assert!(op.edges.contains(&Edge::new(
        Port::Address { point: id, step: 4 },
        Port::Entry(op.input),
        Route::Next
    )));
    assert!(op.edges.contains(&Edge::new(
        Port::Normal(op.input),
        Port::Operation(id),
        Route::Next
    )));
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| op.kind == super::super::OperationKind::Write)
            .count(),
        3
    );
}

#[test]
pub(crate) fn indexed_stores_keep_alias_targets_and_nonreturning_operands_structural() {
    let source = "flag:=false;r:'out{|flag|{'out->xs:=[1];xs[1]=2};|!flag|{'out->xs:=[1];xs[1]=3}}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let ops = checker.paths.values().collect::<Vec<_>>();
    assert_eq!(ops.len(), 2);
    assert_ne!(ops[0].local, ops[1].local);
    assert_eq!(ops[0].storage, ops[1].storage);
    for source in [
        "a:=[[1]];'out{a[{'out.leave();->1}][1]=2}",
        "a:=[[1]];'out{a[1][{'out.leave();->1}]=2}",
        "a:=[[1]];'out{a[1][1]={'out.leave();->2}}",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.paths.len(), 1);
        assert_eq!(checker.scope_exits.len(), 1);
        let (&id, op) = checker.paths.first_key_value().unwrap();
        assert!(
            !op.edges
                .iter()
                .any(|edge| edge.from == Port::Entry(id) && edge.to == Port::Operation(id))
        );
        assert_eq!(
            op.edges
                .iter()
                .filter(|edge| edge.to == Port::Operation(id))
                .count(),
            1
        );
        assert!(op.edges.contains(&Edge::new(
            Port::Normal(op.input),
            Port::Operation(id),
            Route::Next
        )));
    }
}

#[test]
pub(crate) fn indexed_stores_preserve_bounds_types_and_parent_reservations() {
    for (source, code) in [
        ("a:=[[1]];a[0][1]=2", "E101"),
        ("a:=[[1]];a[1][2]=2", "E101"),
        ("a:=[[1]];a[1][true]=2", "E222"),
        ("a:=[[1]];a[1][1]=false", "E207"),
        ("a:[[1]];a[1][1]=2", "E305"),
        ("a:=[[1]];a[{a=[[2]];->1}][1]=3", "E302"),
        ("a:=[[1]];a[1][{a=[[2]];->1}]=3", "E302"),
        ("a:=[[1]];a[1][1]={a=[[2]];->3}", "E302"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn indexed_stores_reject_foreign_repeated_roots_and_mismatched_capacities() {
    let mut checker = check("a:=[[1]];a[1][1]=2");
    let (&id, op) = checker.paths.first_key_value().unwrap();
    let op = op.clone();
    checker.paths.clear();
    checker.path_edges = 0;
    let mut cases = Vec::new();
    let mut steps = op.steps.clone();
    steps[1] = steps[0].clone();
    cases.push(steps);
    let mut steps = op.steps.clone();
    if let Step::Index { point, .. } = &mut steps[0] {
        *point = op.input;
    }
    cases.push(steps);
    let mut steps = op.steps.clone();
    if let Step::Index { capacity, .. } = &mut steps[0] {
        *capacity += 1;
    }
    cases.push(steps);
    for steps in cases {
        assert!(
            checker
                .path_operation(id, op.local, steps, op.input, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(checker.paths.is_empty());
        assert_eq!(checker.path_edges, 0);
    }
    checker.sequence_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .path_operation(id, op.local, op.steps, op.input, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.paths.is_empty());
    assert_eq!(checker.path_edges, 0);
}
