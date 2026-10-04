use super::{super::tests::checked, *};

pub(super) fn keys(reports: &Reports) -> Vec<Key> {
    reports
        .results
        .iter()
        .flat_map(|(&block, (_, result))| {
            result
                .slots
                .iter()
                .flatten()
                .enumerate()
                .flat_map(move |(slot, source)| {
                    let len = match source {
                        Sources::Candidates(values) => values.len(),
                        Sources::Unknown => 0,
                    };
                    (0..len).map(move |position| (block, slot, position))
                })
        })
        .collect()
}

#[test]
pub(crate) fn candidate_inputs_qualify_exact_direct_composed_and_independent_owner_sources() {
    let source = "r:{->7;->z:9;->a:true};copy:'out{{'out->r}};f<int32>:(){->4}";
    let (mut checker, reports) = checked(source);
    let keys = keys(&reports);
    let mut ctx = Context::new(&reports, MAX_EDGES);
    let mut projections = Vec::new();
    let mut owners = BTreeSet::new();
    for key in keys {
        let (owner, input) = checker
            .candidate_input(&mut ctx, key, Span::default())
            .unwrap();
        let op = &checker.emissions[&input.candidate.statement];
        assert_eq!(input.point, op.input);
        assert_eq!(
            op.targets[input.candidate.target].id,
            input.candidate.emission
        );
        assert_eq!(op.targets[input.candidate.target].block, key.0);
        assert_eq!(
            input.projection,
            op.targets[input.candidate.target].projection
        );
        owners.insert(owner);
        projections.push(input.projection);
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    for projection in [
        Projection::Value,
        Projection::Primary,
        Projection::Field(0),
        Projection::Field(1),
    ] {
        assert!(projections.contains(&projection));
    }
    assert_eq!(
        ctx.parts,
        MAX_EDGES - ctx.blocks.len() - ctx.emissions.len()
    );
}

#[test]
pub(crate) fn candidate_inputs_require_initialized_targets_and_exact_membership() {
    for fault in 0..7 {
        let (mut checker, mut reports) = checked("r:{->n:1;->m:2}");
        let key = keys(&reports)[0];
        let candidate = match &mut reports
            .results
            .get_mut(&key.0)
            .unwrap()
            .1
            .slots
            .as_mut()
            .unwrap()[key.1]
        {
            Sources::Candidates(values) => &mut values[key.2],
            _ => panic!(),
        };
        match fault {
            0 => candidate.emission = usize::MAX,
            1 => candidate.target = usize::MAX,
            2 => candidate.statement = usize::MAX,
            3 => {
                let (_, Effect::Emission(op)) =
                    reports.effects.get_mut(&candidate.statement).unwrap()
                else {
                    panic!()
                };
                op.initialized[candidate.target] = false;
            }
            4 => reports.results.get_mut(&key.0).unwrap().0 += 1,
            5 => reports.results.get_mut(&key.0).unwrap().1.consumer = None,
            6 => reports.blocks.get_mut(&key.0).unwrap().1.result = false,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut ctx = Context::new(&reports, MAX_EDGES);
        assert!(
            checker
                .candidate_input(&mut ctx, key, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn candidate_inputs_bound_cached_qualification_scratch() {
    let (mut checker, reports) = checked("r:{->n:1}");
    let key = keys(&reports)[0];
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut ctx = Context::new(&reports, 2);
    let expected = checker
        .candidate_input(&mut ctx, key, Span::default())
        .unwrap();
    assert_eq!(ctx.parts, 0);
    assert_eq!(
        checker
            .candidate_input(&mut ctx, key, Span::default())
            .unwrap(),
        expected
    );
    assert_eq!(ctx.parts, 0);
    for parts in [0, 1] {
        let mut ctx = Context::new(&reports, parts);
        let error = checker
            .candidate_input(&mut ctx, key, Span::default())
            .unwrap_err();
        assert!(error.message.contains("budget"));
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
