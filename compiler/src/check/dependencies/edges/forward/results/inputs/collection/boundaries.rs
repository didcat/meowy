use super::{
    super::{super::tests::checked, tests::keys},
    *,
};

#[test]
pub(crate) fn candidate_inputs_keep_discarded_sources_without_destination_type_inference() {
    let source =
        "d:@\"debug\";flag:=false;r:'out{|flag|{'out->x:\"gone\";d.panic(\"stop\")};->x:3}";
    let (mut checker, reports) = checked(source);
    let inputs: Vec<_> = reports
        .candidate_inputs
        .values()
        .filter(|(_, input)| {
            checker.emissions[&input.candidate.statement].targets[input.candidate.target]
                .field
                .as_deref()
                == Some("x")
        })
        .map(|(_, input)| input)
        .collect();
    assert_eq!(inputs.len(), 2);
    let spans: BTreeSet<_> = inputs
        .iter()
        .map(|input| {
            let span = checker.points[input.point].span;
            &source[span.start..span.end]
        })
        .collect();
    assert_eq!(spans, BTreeSet::from(["\"gone\"", "3"]));
    assert!(
        inputs
            .iter()
            .all(|input| input.projection == Projection::Value)
    );
    assert_eq!(
        checker
            .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap()
            .0,
        reports.candidate_inputs
    );
}

#[test]
pub(crate) fn candidate_inputs_ignore_statement_results_but_require_each_target_visit() {
    let (mut checker, mut reports) = checked("r:{->7;->a:1;->b:true};s:{->r}");
    let expected = reports.candidate_inputs.clone();
    for (_, effect) in reports.effects.values_mut() {
        if let Effect::Emission(op) = effect {
            op.result = false;
        }
    }
    for (_, block) in reports.blocks.values_mut() {
        block.normal = false;
    }
    assert_eq!(
        checker
            .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap()
            .0,
        expected
    );
    let statement = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .map(|(&id, _)| id)
        .unwrap();
    let (_, Effect::Emission(op)) = reports.effects.get_mut(&statement).unwrap() else {
        panic!()
    };
    op.initialized[1] = false;
    let error = checker
        .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap_err();
    assert!(error.message.contains("identity"));
    reports.results = checker.result_sources(&reports, Span::default()).unwrap().0;
    let inputs = checker
        .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap()
        .0;
    assert_eq!(inputs.len(), expected.len() - 1);
    assert!(inputs.values().all(|(_, input)| input.candidate.statement != statement || input.candidate.target != 1));
}

#[test]
pub(crate) fn candidate_inputs_keep_mutable_histories_unknown_and_do_not_publish_missing_positions()
{
    let (mut checker, reports) =
        checked("d:@\"debug\";flag:=false;r:'out{|flag|{'out->x:=1;x=2;d.panic(\"stop\")};->x:3}");
    let (&block, (_, row)) = reports
        .results
        .iter()
        .find(|(_, (_, row))| {
            row.slots
                .as_ref()
                .is_some_and(|slots| slots.contains(&Sources::Unknown))
        })
        .unwrap();
    let slot = row
        .slots
        .as_ref()
        .unwrap()
        .iter()
        .position(|source| *source == Sources::Unknown)
        .unwrap();
    assert!(
        reports
            .candidate_inputs
            .keys()
            .all(|key| (key.0, key.1) != (block, slot))
    );
    for key in [
        (block, slot, 0),
        (block, usize::MAX, 0),
        (block, 0, usize::MAX),
    ] {
        let mut ctx = Context::new(&reports, MAX_EDGES);
        assert!(
            checker
                .candidate_input(&mut ctx, key, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    assert_eq!(
        reports.candidate_inputs.keys().copied().collect::<Vec<_>>(),
        keys(&reports)
    );
}
