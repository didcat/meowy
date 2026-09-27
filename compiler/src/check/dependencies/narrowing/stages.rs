use super::{tests::check, *};

#[test]
pub(crate) fn narrowing_stages_follow_raw_local_and_field_results_without_bypasses() {
    let source = "f:(v<int32><null>,r<{n<int32><null>}>){|v<int32>|a:v;|r.n<int32>|b:r.n}";
    crate::compile(source).unwrap();
    let checker = check(source);
    for (&id, op) in &checker.narrowings {
        let mut expected = vec![Edge::new(
            Port::Entry(id),
            Port::Entry(op.input),
            Route::Next,
        )];
        if op.changed {
            expected.extend([
                Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]);
        } else {
            expected.push(Edge::new(
                Port::Normal(op.input),
                Port::Normal(id),
                Route::Next,
            ));
        }
        assert_eq!(op.edges, expected);
        if let Some(field) = checker.fields.get(&op.input) {
            assert_eq!(field.edges.last().unwrap().to, Port::Normal(op.input));
        }
    }
    assert_eq!(
        checker.narrowings.values().filter(|op| op.changed).count(),
        2
    );
}

#[test]
pub(crate) fn narrowing_stages_preserve_effectful_receivers_and_outer_expected_contexts() {
    let source = "f<{n<int32>}>:(){->{->n:1}};x<int32><null>:f().n";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, op) = checker.narrowings.first_key_value().unwrap();
    let field = &checker.fields[&op.input];
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(checker.invocations.len(), 1);
    assert_eq!(field.input, call.point);
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    assert!(!op.changed);
    assert_eq!(op.edges.last().unwrap().to, Port::Normal(id));
    let outer = checker.points[id].parent.unwrap();
    assert_eq!(checker.coercions[&outer].input, id);
    assert_eq!(
        checker.coercions[&outer].kind,
        super::super::CoercionKind::Convert
    );
}

#[test]
pub(crate) fn narrowing_stages_omit_never_results_before_and_after_conversion() {
    for source in [
        "f<never>:(v<never>){->v}",
        "f<never>:(r<{n<never>}>){->r.n}",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        for (&id, op) in checker.narrowings.iter().filter(|(_, op)| !op.normal) {
            assert!(!op.changed);
            assert_eq!(
                op.edges,
                [Edge::new(
                    Port::Entry(id),
                    Port::Entry(op.input),
                    Route::Next
                )]
            );
        }
        assert!(checker.narrowings.values().any(|op| !op.normal));
    }
    let mut checker = Checker::new();
    let ty = hir::Type::union(vec![hir::Type::Bool, hir::Type::Null]);
    checker.tags.insert(
        ((0, Vec::new()), ty.clone()),
        vec![
            (hir::Type::Bool, crate::flow::FALSE),
            (hir::Type::Null, crate::flow::FALSE),
        ],
    );
    let span = Span::new(1, 2);
    let (id, value) = checker
        .with_point_id(PointKind::Expr, span, |checker| {
            checker.narrow_source(span, |_| {
                Ok(hir::Expr {
                    kind: hir::ExprKind::Local(0),
                    ty,
                    span,
                })
            })
        })
        .unwrap();
    assert_eq!(value.ty, hir::Type::Never);
    let op = &checker.narrowings[&id];
    assert!(op.changed && !op.normal);
    assert_eq!(op.edges.len(), 2);
    assert_eq!(op.edges.last().unwrap().to, Port::Operation(id));
}

#[test]
pub(crate) fn narrowing_stages_keep_mutable_observations_control_and_type_rules() {
    let source = "flag:false;v<int32><null>:=1;|flag|v;|v<int32>|x:v";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert!(checker.narrowings.values().any(|op| op.control));
    assert!(!checker.proofs.observations.is_empty());
    for (source, code) in [
        ("f:(v<int32><null>){x<int32>:v}", "E207"),
        ("f:(v<int32><null>){x:v~<int32>}", "E208"),
        ("v<int32><null>:=1;|v<int32>|{v=null;x<int32>:v}", "E207"),
        ("v:=1;p:&v;v=2;after:*p", "E302"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn narrowing_stages_validate_source_identity_and_shared_budget_atomically() {
    let mut checker = check("v:1;x:v");
    let (&id, op) = checker.narrowings.first_key_value().unwrap();
    let op = op.clone();
    let count = checker.narrowing_edges;
    checker
        .capture_narrowing(id, op.input, op.changed, op.normal, op.span)
        .unwrap();
    assert_eq!(checker.narrowing_edges, count);
    assert!(
        checker
            .capture_narrowing(id, op.input, !op.changed, op.normal, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.narrowings[&id], op);
    checker.narrowings.clear();
    checker.narrowing_edges = 0;
    checker.points[op.input].parent = None;
    assert!(
        checker
            .capture_narrowing(id, op.input, op.changed, op.normal, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[op.input].parent = Some(id);
    checker.index_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .capture_narrowing(id, op.input, op.changed, op.normal, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.narrowings.is_empty());
    assert_eq!(checker.narrowing_edges, 0);
}
