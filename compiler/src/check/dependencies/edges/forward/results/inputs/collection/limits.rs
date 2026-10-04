use super::{
    super::{super::tests::checked, tests::keys},
    *,
};

#[test]
pub(crate) fn candidate_inputs_share_exact_map_scratch_and_work_limits() {
    let (mut checker, reports) = checked("r:{->1;->n:2};s:{->r}");
    let expected = &reports.candidate_inputs;
    let base = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + reports.slot_uses.len();
    let limit = base + expected.len();
    let scratch = reports.results.len()
        + expected
            .values()
            .map(|(_, input)| input.candidate.statement)
            .collect::<BTreeSet<_>>()
            .len()
        + expected
            .values()
            .filter(|(_, input)| input.source.is_some())
            .map(|(_, input)| input.candidate.statement)
            .collect::<BTreeSet<_>>()
            .len();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let work = checker.flow.work;
    assert_eq!(
        checker
            .candidate_inputs_limited(&reports, Span::default(), limit, scratch)
            .unwrap(),
        (expected.clone(), 0)
    );
    let work = checker.flow.work - work;
    for (limit, parts) in [
        (limit - 1, scratch),
        (base - 1, scratch),
        (limit, scratch - 1),
    ] {
        assert!(
            checker
                .candidate_inputs_limited(&reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.candidate_inputs_limited(&reports, Span::default(), limit, scratch);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(result) = result {
            assert_eq!(result, (expected.clone(), 0));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn candidate_inputs_reject_late_corrupt_candidates_and_reports_atomically() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked("a:{->1};b:{->2;->n:3};c:{->b}");
        let key = *keys(&reports).last().unwrap();
        let input = reports.candidate_inputs[&key].1;
        let statement = input.candidate.statement;
        match fault {
            0 => reports
                .results
                .get_mut(&key.0)
                .unwrap()
                .1
                .slots
                .as_mut()
                .unwrap()
                .pop()
                .map(|_| ())
                .unwrap(),
            1 => reports.results.get_mut(&key.0).unwrap().0 += 1,
            2 => reports.blocks.get_mut(&key.0).unwrap().1.result = false,
            3 => checker.points[input.point].parent = None,
            4 => {
                checker.emission_sources.remove(&input.candidate.emission);
            }
            5 => reports.effects.get_mut(&statement).unwrap().0 += 1,
            6..=9 => {
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&statement).unwrap() else {
                    panic!()
                };
                match fault {
                    6 => op.initialized[input.candidate.target] = false,
                    7 => op.input = usize::MAX,
                    8 => op.targets[input.candidate.target].field = Some("wrong".to_owned()),
                    9 => op.control = !op.control,
                    _ => unreachable!(),
                }
            }
            10 | 11 => {
                let Sources::Candidates(values) = &mut reports
                    .results
                    .get_mut(&key.0)
                    .unwrap()
                    .1
                    .slots
                    .as_mut()
                    .unwrap()[key.1]
                else {
                    panic!()
                };
                if fault == 10 {
                    values[key.2].target = usize::MAX;
                } else {
                    values[key.2].emission = usize::MAX;
                }
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap_err();
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn candidate_inputs_validate_wide_compositions_once_per_pass() {
    let mut costs = Vec::new();
    for width in [16, 32] {
        let fields = (0..width)
            .map(|n| format!("->f{n:02}:{n};"))
            .collect::<String>();
        let source = format!("r:{{{fields}}};s:{{->r}}");
        let (mut checker, reports) = checked(&source);
        let scratch = reports.results.len()
            + reports
                .candidate_inputs
                .values()
                .map(|(_, input)| input.candidate.statement)
                .collect::<BTreeSet<_>>()
                .len()
            + reports
                .candidate_inputs
                .values()
                .filter(|(_, input)| input.source.is_some())
                .map(|(_, input)| input.candidate.statement)
                .collect::<BTreeSet<_>>()
                .len();
        let work = checker.flow.work;
        let (inputs, remaining) = checker
            .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, scratch)
            .unwrap();
        costs.push(checker.flow.work - work);
        assert_eq!(remaining, 0);
        assert_eq!(inputs, reports.candidate_inputs);
        let mut ctx = Context::new(&reports, MAX_EDGES);
        for key in keys(&reports) {
            checker
                .candidate_input(&mut ctx, key, Span::default())
                .unwrap();
        }
        assert_eq!(ctx.emissions.len(), width + 1);
    }
    assert!(costs[1] > costs[0]);
    assert!(costs[1] < costs[0] * 3, "{costs:?}");
}
