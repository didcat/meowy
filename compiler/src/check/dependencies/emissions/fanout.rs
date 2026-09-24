use super::{tests::check, *};

#[test]
pub(crate) fn emission_fanout_preserves_original_roots_projections_and_component_order() {
    let source = "source:{->7;->a:1;->b:true};copy:{->source}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&point, emission) = checker
        .emissions
        .iter()
        .find(|(_, emission)| emission.targets.len() == 3)
        .unwrap();
    assert_eq!(
        &source[checker.points[emission.input].span.start..checker.points[emission.input].span.end],
        "source"
    );
    assert_eq!(
        emission
            .targets
            .iter()
            .map(|target| target.projection)
            .collect::<Vec<_>>(),
        [
            Projection::Primary,
            Projection::Field(0),
            Projection::Field(1)
        ]
    );
    assert_eq!(emission.targets[1].field.as_deref(), Some("a"));
    assert_eq!(emission.targets[2].field.as_deref(), Some("b"));
    assert_eq!(emission.edges.len(), 5);
    for (index, target) in emission.targets.iter().enumerate() {
        assert_eq!(checker.emission_sources[&target.id], (point, index));
        assert_eq!(emission.edges[index + 1].to, Port::Emission(target.id));
    }
    assert_eq!(emission.edges.last().unwrap().to, Port::Normal(point));
}

#[test]
pub(crate) fn emission_fanout_keeps_contextual_records_outer_targets_and_reference_sources() {
    let source = "x:1;source:{->view:&x};copy<{view<&int32>}>:'out{{'out->{->source}}}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let fanout = checker
        .emissions
        .iter()
        .filter(|(_, emission)| emission.targets.len() > 1)
        .collect::<Vec<_>>();
    assert!(!fanout.is_empty());
    assert!(
        fanout
            .iter()
            .any(|(point, emission)| checker.points[**point].block
                != Some(emission.targets[0].block))
    );
    assert!(
        fanout
            .iter()
            .all(|(_, emission)| checker.points[emission.input].complete)
    );
    assert!(!checker.record_pointees.is_empty());
}

#[test]
pub(crate) fn emission_fanout_projection_checks_reject_foreign_cells_and_field_indices() {
    let mut checker = Checker::new();
    let ty = hir::Type::Record {
        primary: Box::new(hir::Type::Null),
        fields: vec![hir::Field {
            name: "x".into(),
            ty: hir::Type::Bool,
            mutable: false,
        }],
    };
    let cell = checker.local(ty.clone());
    let span = Span::default();
    let base = hir::Expr {
        kind: hir::ExprKind::Local(cell),
        ty,
        span,
    };
    let mut value = hir::Expr {
        kind: hir::ExprKind::Field {
            value: Box::new(base),
            index: 0,
        },
        ty: hir::Type::Bool,
        span,
    };
    assert_eq!(
        checker
            .emission_projection(Some(cell), &value, Some("x"), span)
            .unwrap(),
        Projection::Field(0)
    );
    assert!(
        checker
            .emission_projection(Some(cell), &value, Some("other"), span)
            .is_err()
    );
    let hir::ExprKind::Field { index, .. } = &mut value.kind else {
        panic!()
    };
    *index = 1;
    assert!(
        checker
            .emission_projection(Some(cell), &value, Some("x"), span)
            .is_err()
    );
    let hir::ExprKind::Field { value: base, index } = &mut value.kind else {
        panic!()
    };
    *index = 0;
    base.kind = hir::ExprKind::Local(cell + 1);
    assert!(
        checker
            .emission_projection(Some(cell), &value, Some("x"), span)
            .is_err()
    );
}

#[test]
pub(crate) fn emission_fanout_publishes_all_targets_atomically_and_rejects_duplicates() {
    let mut checker = Checker::new();
    let block = crate::parser::parse("row:{->a:1;->b:true};->row").unwrap();
    checker.block_start(&block, None, None, false).unwrap();
    checker.stmt(&block.stmts[0]).unwrap();
    let (point, stmts) = checker.checked_stmt(&block.stmts[1]).unwrap();
    let hir::Stmt::Bind { id: from, .. } = &stmts[0] else {
        panic!()
    };
    let emission = checker.emissions.remove(&point).unwrap();
    checker.emission_edges -= emission.edges.len();
    for target in &emission.targets {
        checker.emission_sources.remove(&target.id);
    }
    let count = checker.emission_edges;
    checker.sequence_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .emission_operation(point, emission.input, Some(*from), &stmts, emission.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(!checker.emissions.contains_key(&point));
    assert!(
        emission
            .targets
            .iter()
            .all(|target| !checker.emission_sources.contains_key(&target.id))
    );
    assert_eq!(checker.emission_edges, count);
    checker.sequence_edges = 0;
    checker
        .emission_operation(point, emission.input, Some(*from), &stmts, emission.span)
        .unwrap();
    assert!(
        emission
            .targets
            .iter()
            .all(|target| checker.emission_sources[&target.id].0 == point)
    );
    let mut duplicate = stmts.clone();
    duplicate.push(stmts[1].clone());
    assert!(
        checker
            .emission_operation(
                point,
                emission.input,
                Some(*from),
                &duplicate,
                emission.span
            )
            .unwrap_err()
            .message
            .contains("identity mismatch")
    );
    assert_eq!(checker.emissions[&point].targets, emission.targets);
}
