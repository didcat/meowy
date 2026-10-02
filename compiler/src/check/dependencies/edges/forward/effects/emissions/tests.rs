use super::{super::tests::checked, *};

#[test]
pub(crate) fn emission_effects_keep_targets_projections_composition_and_alias_storage() {
    for source in [
        "row:'out{->name:=1;{'out->2};name=3}",
        "source:{->7;->a:1;->b:true};copy:{->source}",
        "x:1;source:{->view:&x};copy<{view<&int32>}>:'out{{'out->{->source}}}",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        for (&id, op) in &checker.emissions {
            assert_eq!(
                reports.effects[&id],
                (
                    op.owner,
                    Effect::Emission(Observed {
                        input: op.input,
                        composed: op.composed,
                        targets: op.targets.clone(),
                        control: op.control,
                        initialized: vec![true; op.targets.len()],
                        result: true,
                    })
                )
            );
            assert!(!reports.index.operations.contains_key(&id));
        }
    }
}

#[test]
pub(crate) fn emission_effects_observe_each_target_and_statement_result_independently() {
    let (mut checker, mut reports) = checked("source:{->7;->a:1;->b:true};copy:{->source}", false);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.targets.len() == 3)
        .unwrap();
    let ports: Vec<_> = op
        .targets
        .iter()
        .map(|target| Port::Emission(target.id))
        .chain([Port::Normal(id)])
        .collect();
    for (index, port) in ports.into_iter().enumerate() {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 8, 0)
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Emission(op)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(
            op.initialized,
            (0..3).map(|part| part == index).collect::<Vec<_>>()
        );
        assert_eq!(op.result, index == 3);
    }
}

#[test]
pub(crate) fn emission_effects_do_not_publish_blocks_or_skip_stopped_inputs() {
    let source = "d:@\"debug\";row:{->1;d.panic(\"stop\")};after:{->2}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.emissions.len(), 2);
    for (&id, op) in &checker.emissions {
        let observed = reports.effects.get(&id);
        let input = &checker.points[op.input];
        if &source[input.span.start..input.span.end] == "1" {
            let Some((_, Effect::Emission(observed))) = observed else {
                panic!()
            };
            assert!(observed.initialized[0] && observed.result);
            assert!(
                !reports.entries[&0]
                    .1
                    .ports
                    .contains(&Port::BlockResult(op.targets[0].block))
            );
        } else {
            assert!(observed.is_none());
        }
    }
    let (checker, reports) = checked("'out{->{'out.leave()}};-><T>:<uint8>", false);
    assert!(checker.emissions.is_empty());
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Emission(_)))
    );
}

#[test]
pub(crate) fn emission_effects_preserve_independent_owners_and_control() {
    let source = "flag:false;|flag|->1;f<int32>:(n<int32>){->n}";
    let (checker, reports) = checked(source, true);
    assert!(checker.emissions.values().any(|op| op.control));
    assert!(checker.emissions.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.emissions {
        let (owner, Effect::Emission(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.control, op.control);
        assert_eq!(observed.input, op.input);
    }
}
