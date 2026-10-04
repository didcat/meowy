use super::{super::super::tests::checked, limits::SOURCE, *};

#[test]
pub(crate) fn candidate_sources_keep_sparse_initialization_and_independent_results() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
    let targets = op.targets.clone();
    for result in [false, true] {
        for selected in [None, Some(0), Some(2)] {
            let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.initialized.fill(false);
            op.result = result;
            if let Some(part) = selected {
                op.initialized[part] = true;
            }
            for (_, block) in reports.blocks.values_mut() {
                block.normal = false;
            }
            reports.results = checker.result_sources(&reports, Span::default()).unwrap().0;
            let before = format!("{reports:?}");
            let (inputs, _) = checker
                .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap();
            let projected: Vec<_> = inputs
                .values()
                .filter(|(_, input)| input.candidate.statement == id)
                .collect();
            assert_eq!(projected.len(), usize::from(selected.is_some()));
            if let Some(part) = selected {
                let (_, input) = projected[0];
                assert_eq!(input.candidate.emission, targets[part].id);
                assert_eq!(input.source.unwrap().index, part);
            }
            assert_eq!(format!("{reports:?}"), before);
        }
    }
}

#[test]
pub(crate) fn candidate_sources_validate_unobserved_source_layout_suffixes() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
    let block = op.targets[0].block;
    let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.initialized = vec![true, false, false];
    op.targets[2].field = Some("wrong".into());
    checker.emissions.get_mut(&id).unwrap().targets[2].field = Some("wrong".into());
    for slot in &mut reports
        .results
        .get_mut(&block)
        .unwrap()
        .1
        .slots
        .as_mut()
        .unwrap()[1..]
    {
        *slot = Sources::Unknown;
    }
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let error = checker
        .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap_err();
    assert!(
        error.message.contains("emission-slot identity"),
        "{error:?}"
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn candidate_sources_keep_unknown_empty_and_multiple_source_histories() {
    for source in [
        "d:@\"debug\";flag:=false;r:'out{|flag|{'out->{->n:=1};d.panic(\"stop\")};->n:2}",
        "r<{}>:{};s:{->r}",
        "flag:=false;r:{|flag|->n:1;|!flag|->n:2};s:{->r}",
    ] {
        let (mut checker, reports) = checked(source);
        let sources: Vec<_> = reports
            .candidate_inputs
            .values()
            .filter_map(|(_, input)| input.source)
            .map(|slot| &reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index])
            .collect();
        assert!(!sources.is_empty(), "{source}");
        if source.contains("n:=") {
            assert!(sources.contains(&&Sources::Unknown));
        } else if source.contains("|flag|") {
            assert!(
                sources.iter().any(
                    |source| matches!(source, Sources::Candidates(values) if values.len() == 2)
                )
            );
        } else {
            assert!(
                sources.iter().all(
                    |source| matches!(source, Sources::Candidates(values) if values.is_empty())
                )
            );
        }
        let before = format!("{reports:?}");
        assert_eq!(
            checker
                .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap()
                .0,
            reports.candidate_inputs
        );
        assert_eq!(format!("{reports:?}"), before);
    }
}
