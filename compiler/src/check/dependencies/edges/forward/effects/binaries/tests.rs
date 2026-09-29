use super::{super::tests::checked, *};

#[test]
pub(crate) fn binary_effect_stages_preserve_scalars_projections_and_result_routes() {
    for source in [
        "a:{->7;->tag:true};b:{->2;->tag:false};x:a+b",
        "x:7.0/2.0",
        "x:\"a\"<\"b\"",
        "x:null==null",
        "x:false!=true",
        "b:@\"bits\";x:b.xor(7,2)",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        let op = op.clone();
        for (port, kind) in [
            (Port::Operation(id), Kind::Operation),
            (Port::Normal(id), Kind::Result),
        ] {
            let stage = checker
                .binary_effect_stage(&reports, 0, port, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(
                (stage.inputs, stage.types, stage.plan, stage.kind),
                (op.inputs, op.types, op.plan, kind)
            );
            assert_eq!(stage.op, op.op);
        }
        for step in 0..2 {
            assert_eq!(
                checker
                    .binary_effect_stage(
                        &reports,
                        0,
                        Port::Projection { point: id, step },
                        Span::default()
                    )
                    .is_ok(),
                op.plan.primary[step]
            );
        }
    }
}

#[test]
pub(crate) fn binary_effect_stages_preserve_partial_primaries_and_opaque_comparisons() {
    for (tail, step) in [("r+1", 0), ("1+r", 1)] {
        let source = format!("f<never>:(r<{{-><never>;tag<boolean>}}> ){{->{tail}}}");
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source, false);
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        let owner = op.owner;
        let stage = checker
            .binary_effect_stage(
                &reports,
                owner,
                Port::Projection { point: id, step },
                Span::default(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(stage.types.inputs[step], Class::Never);
        assert_eq!(stage.types.result, Class::Never);
        assert!(
            checker
                .binary_effect_stage(&reports, owner, Port::Operation(id), Span::default())
                .is_err()
        );
    }
    for source in [
        "a:{->n:1};b:{->n:2};x:a==b",
        "a:[1];b:[2];x:a==b",
        "n:1;a:&n;b:&n;x:a==b",
        "a<int32><null>:1;b<int32><null>:null;x:a==b",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let id = *checker.binaries.first_key_value().unwrap().0;
        assert!(
            checker
                .binary_effect_stage(&reports, 0, Port::Operation(id), Span::default())
                .unwrap()
                .is_none()
        );
        assert_eq!(reports.effects[&id].1, Effect::Unknown);
    }
}

#[test]
pub(crate) fn binary_effect_stages_require_the_sequence_link_and_registered_results() {
    let (mut checker, mut reports) = checked("a:{->1;->tag:true};b:{->2;->tag:false};a+b", false);
    let id = *checker.binaries.first_key_value().unwrap().0;
    let key = SequenceSource::Expr(id);
    let edge = checker
        .sequences
        .get_mut(&key)
        .unwrap()
        .edges
        .pop()
        .unwrap();
    assert!(
        checker
            .binary_effect_stage(&reports, 0, Port::Operation(id), Span::default())
            .is_err()
    );
    checker.sequences.get_mut(&key).unwrap().edges.push(edge);
    reports.index.operations.remove(&id);
    assert!(
        checker
            .binary_effect_stage(&reports, 0, Port::Normal(id), Span::default())
            .is_err()
    );
    assert!(
        checker
            .binary_effect_stage(
                &reports,
                0,
                Port::Projection { point: id, step: 0 },
                Span::default()
            )
            .unwrap()
            .is_some()
    );
}

#[test]
pub(crate) fn binary_effect_stages_bound_work_and_exclude_short_circuit_roots() {
    let (mut checker, reports) = checked("false&&(1<2)", false);
    let branch = *checker.branch_edges.first_key_value().unwrap().0;
    assert!(
        checker
            .binary_effect_stage(&reports, 0, Port::Normal(branch), Span::default())
            .unwrap()
            .is_none()
    );
    let id = *checker.binaries.first_key_value().unwrap().0;
    let before = checker.flow.work;
    let expected = checker
        .binary_effect_stage(&reports, 0, Port::Normal(id), Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.binary_effect_stage(&reports, 0, Port::Normal(id), Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(stage) = result {
            assert_eq!(stage, expected);
        }
    }
}
