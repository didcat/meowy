use super::{tests::check, *};

#[test]
pub(crate) fn emission_composition_retains_exact_local_and_field_count() {
    for source in [
        "source:{->7;->a:1;->b:true};copy:{->source}",
        "source:{->a:1;->b:true};copy:{->source}",
        "x:1;source:{->view:&x};copy<{view<&int32>}>:'out{{'out->{->source}}}",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let mut count = 0;
        for emission in checker.emissions.values() {
            if let Some(composed) = emission.composed {
                count += 1;
                let hir::Type::Record { fields, .. } = &checker.locals[composed.local] else {
                    panic!()
                };
                assert_eq!(composed.count, fields.len());
                assert_eq!(emission.targets.len(), composed.count + 1);
                assert_eq!(emission.targets[0].projection, Projection::Primary);
                for (index, target) in emission.targets.iter().skip(1).enumerate() {
                    assert_eq!(target.projection, Projection::Field(index));
                    assert_eq!(target.field.as_deref(), Some(fields[index].name.as_str()));
                    assert!(target.alias.is_none() && target.storage.is_none());
                }
            } else {
                assert_eq!(emission.targets.len(), 1);
                assert_eq!(emission.targets[0].projection, Projection::Value);
            }
        }
        assert!(count > 0);
    }
}

#[test]
pub(crate) fn emission_composition_keeps_direct_aliases_and_stopped_inputs_separate() {
    let source = "row:'out{->name:=1;{'out->2};name=3};'stop{->{'stop.leave()}}";
    crate::compile(source).unwrap();
    let checker = check(source);
    assert_eq!(checker.emissions.len(), 2);
    for emission in checker.emissions.values() {
        assert!(emission.composed.is_none());
        let target = &emission.targets[0];
        if let Some(alias) = target.alias {
            assert_eq!(target.storage, Some(checker.proofs.aliases[&alias].root));
        }
    }
}

#[test]
pub(crate) fn emission_composition_bounds_checked_shapes_without_copying_types() {
    let mut checker = Checker::new();
    assert_eq!(
        checker.emission_composition(None, Span::default()).unwrap(),
        None
    );
    assert!(
        checker
            .emission_composition(Some(0), Span::default())
            .is_err()
    );
    let scalar = checker.local(hir::Type::Bool);
    assert!(
        checker
            .emission_composition(Some(scalar), Span::default())
            .is_err()
    );
    for count in [0, MAX_TARGETS - 1, MAX_TARGETS] {
        let local = checker.local(hir::Type::Record {
            primary: Box::new(hir::Type::Null),
            fields: vec![
                hir::Field {
                    name: "x".into(),
                    ty: hir::Type::Bool,
                    mutable: false
                };
                count
            ],
        });
        let result = checker.emission_composition(Some(local), Span::default());
        if count < MAX_TARGETS {
            assert_eq!(result.unwrap(), Some(Composition { local, count }));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
}

#[test]
pub(crate) fn emission_composition_replays_reject_changed_context_without_publication() {
    let mut checker = Checker::new();
    let block = crate::parser::parse("row:{->a:1};->row").unwrap();
    checker.block_start(&block, None, None, false).unwrap();
    checker.stmt(&block.stmts[0]).unwrap();
    let (id, stmts) = checker.checked_stmt(&block.stmts[1]).unwrap();
    let op = &checker.emissions[&id];
    let input = op.input;
    let span = op.span;
    let composed = op.composed.unwrap();
    let sources = checker.emission_sources.clone();
    let edges = checker.emission_edges;
    for prior in [
        None,
        Some(Composition {
            local: composed.local + 1,
            ..composed
        }),
        Some(Composition {
            count: composed.count + 1,
            ..composed
        }),
    ] {
        checker.emissions.get_mut(&id).unwrap().composed = prior;
        assert!(
            checker
                .emission_operation(id, input, Some(composed.local), &stmts, span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(checker.emissions[&id].composed, prior);
        assert_eq!(checker.emission_sources, sources);
        assert_eq!(checker.emission_edges, edges);
    }
}
