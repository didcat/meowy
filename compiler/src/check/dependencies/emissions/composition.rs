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
            assert_eq!(
                result.unwrap(),
                Some(Composition {
                    local,
                    count,
                    shared_primary: None,
                })
            );
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
        Some(Composition {
            shared_primary: Some(ScalarKind::Bool),
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

#[test]
pub(crate) fn emission_composition_captures_shared_scalar_primaries_and_owners() {
    for (ty, value, kind) in [
        (
            "int8",
            "1",
            ScalarKind::Int {
                bits: 8,
                signed: true,
            },
        ),
        (
            "uint64",
            "1",
            ScalarKind::Int {
                bits: 64,
                signed: false,
            },
        ),
        ("float32", "1.5", ScalarKind::Float { bits: 32 }),
        ("boolean", "true", ScalarKind::Bool),
        ("string", "\"cat\"", ScalarKind::String),
        ("null", "null", ScalarKind::Null),
    ] {
        let source = format!(
            "n<{ty}>:{value};p:&n;out:{{->p;->tag:true}}.{{copy:{{->(($))}}}};f<null>:(p<&{ty}>){{out:p.{{->$;->tag:true}}.{{copy:{{->$}}}}}}"
        );
        crate::compile(&source).unwrap();
        let checker = check(&source);
        let mut owners = std::collections::BTreeSet::new();
        for op in checker
            .emissions
            .values()
            .filter(|op| op.composed.is_some())
        {
            let composed = op.composed.unwrap();
            assert_eq!(composed.shared_primary, Some(kind));
            assert_eq!(composed.count, 1);
            assert_eq!(op.targets[0].projection, Projection::Primary);
            assert_eq!(op.targets[1].projection, Projection::Field(0));
            assert!(
                op.targets
                    .iter()
                    .all(|target| target.alias.is_none() && target.storage.is_none())
            );
            owners.insert(op.owner);
        }
        assert_eq!(owners, std::collections::BTreeSet::from([0, 1]));
    }
}

#[test]
pub(crate) fn emission_composition_keeps_other_primaries_without_scalar_descriptors() {
    for source in [
        "n:1;r:{->n;->tag:true};copy:{->r}",
        "n:[1];r:{->&n;->tag:true};copy:{->r}",
        "n<int32><null>:1;r:{->&n;->tag:true};copy:{->r}",
        "n:1;p:&n;r:{->&p;->tag:true};copy:{->r}",
        "n:@\"memory\".heap;r:{->&n;->tag:true};copy:{->r}",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let ops: Vec<_> = checker
            .emissions
            .values()
            .filter_map(|op| op.composed)
            .collect();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].shared_primary, None, "{source}");
    }
}

#[test]
pub(crate) fn emission_composition_shared_capture_preserves_exact_work_and_atomic_replay() {
    let mut costs = Vec::new();
    for value in ["n", "&n"] {
        let mut checker = Checker::new();
        let block =
            crate::parser::parse(&format!("n:1;row:{{->{value};->tag:true}};->row")).unwrap();
        checker.block_start(&block, None, None, false).unwrap();
        for stmt in &block.stmts[..2] {
            checker.stmt(stmt).unwrap();
        }
        let (id, stmts) = checker.checked_stmt(&block.stmts[2]).unwrap();
        let op = &checker.emissions[&id];
        let (input, span, composed) = (op.input, op.span, op.composed.unwrap());
        let edges = checker.edge_counts();
        let sources = checker.emission_sources.clone();
        let start = checker.flow.work;
        checker
            .emission_operation(id, input, Some(composed.local), &stmts, span)
            .unwrap();
        let work = checker.flow.work - start;
        costs.push(work);
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result = checker.emission_operation(id, input, Some(composed.local), &stmts, span);
            assert_eq!(result.is_ok(), short == 0);
            assert_eq!(checker.emissions[&id].composed, Some(composed));
        }
        checker.flow.work = 0;
        checker.flow.full = false;
        let changed = Composition {
            shared_primary: Some(ScalarKind::Bool),
            ..composed
        };
        checker.emissions.get_mut(&id).unwrap().composed = Some(changed);
        assert!(
            checker
                .emission_operation(id, input, Some(composed.local), &stmts, span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(checker.emissions[&id].composed, Some(changed));
        assert_eq!(checker.emission_sources, sources);
        assert_eq!(checker.edge_counts(), edges);
    }
    assert_eq!(costs[0], costs[1]);
}
