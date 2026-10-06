use super::*;

#[test]
pub(crate) fn dispatch_candidate_graphs_reject_stale_slots_and_origins_atomically() {
    for fault in 0..11 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&key, &(_, input)) = reports
            .candidate_inputs
            .iter()
            .rev()
            .find(|(_, (_, input))| input.source.is_some())
            .unwrap();
        let port = Port::Emission(input.candidate.emission);
        let slot = input.source.unwrap();
        let dispatch = reports.results[&slot.block].1.dispatch.unwrap();
        match fault {
            0 => reports.slot_uses.get_mut(&port).unwrap().0 += 1,
            1 => reports.slot_uses.get_mut(&port).unwrap().1.block = usize::MAX,
            2 => reports.slot_uses.get_mut(&port).unwrap().1.block = key.0,
            3 => reports.slot_uses.get_mut(&port).unwrap().1.index += 1,
            4 => reports.slot_uses.get_mut(&port).unwrap().1.index = usize::MAX,
            5 => reports.results.get_mut(&slot.block).unwrap().1.dispatch = None,
            6 => reports.results.get_mut(&slot.block).unwrap().1.consumer = Some(dispatch),
            7 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            8 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
            9 => checker.dispatch_ops.get_mut(&dispatch).unwrap().owner += 1,
            10 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                slots[2].field = Some("wrong".into());
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}{:?}", checker.bodies, checker.edge_counts());
        let errors = [
            checker
                .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .err()
                .unwrap(),
            checker
                .candidate_graph(&reports, Span::default())
                .err()
                .unwrap(),
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .err()
                .unwrap(),
        ];
        assert!(
            errors
                .iter()
                .all(|error| error.message.contains("identity")),
            "{fault}: {errors:?}"
        );
        assert_eq!(
            format!("{reports:?}{:?}{:?}", checker.bodies, checker.edge_counts()),
            before
        );
    }
}

#[test]
pub(crate) fn dispatch_candidate_sources_share_one_qualified_source_cache_entry() {
    let (mut checker, reports) = checked(SOURCE);
    let keys: Vec<_> = reports
        .candidate_inputs
        .iter()
        .filter(|(_, (_, input))| input.source.is_some())
        .map(|(&key, _)| key)
        .collect();
    assert_eq!(keys.len(), 3);
    let before = format!("{reports:?}");
    let mut ctx = Context::new(&reports, 3);
    for key in &keys {
        assert_eq!(
            checker
                .candidate_input(&mut ctx, *key, Span::default())
                .unwrap(),
            reports.candidate_inputs[key]
        );
    }
    assert_eq!(
        (
            ctx.parts,
            ctx.sources.len(),
            ctx.emissions.len(),
            ctx.blocks.len()
        ),
        (0, 1, 1, 1)
    );
    let mut ctx = Context::new(&reports, 2);
    assert!(
        checker
            .candidate_input(&mut ctx, keys[0], Span::default())
            .unwrap_err()
            .message
            .contains("candidate-source budget")
    );
    assert!(ctx.sources.is_empty());
    assert_eq!(format!("{reports:?}"), before);
}

#[test]
pub(crate) fn dispatch_candidate_graphs_bound_map_scratch_and_work_exactly() {
    for mode in 0..3 {
        let (mut checker, reports) = checked(SOURCE);
        let limit = reports.effects.len()
            + reports.blocks.len()
            + reports.results.len()
            + reports.consumers.len()
            + reports.eligible.len()
            + reports.initializers.len()
            + reports.slot_uses.len()
            + reports.candidate_inputs.len();
        let run = |checker: &mut Checker, parts| match mode {
            0 => checker
                .candidate_inputs_limited(&reports, Span::default(), limit, parts)
                .map(|(inputs, parts)| {
                    assert_eq!(inputs, reports.candidate_inputs);
                    parts
                }),
            1 => checker
                .candidate_graph_limited(&reports, Span::default(), parts)
                .map(|(_, parts)| parts),
            2 => checker
                .direct_graph(&reports, Span::default(), parts)
                .map(|(_, parts)| parts),
            _ => unreachable!(),
        };
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let start = checker.flow.work;
        let left = run(&mut checker, MAX_EDGES).unwrap();
        let work = checker.flow.work - start;
        let scratch = MAX_EDGES - left;
        assert_eq!(run(&mut checker, scratch).unwrap(), 0);
        assert!(
            run(&mut checker, scratch - 1)
                .unwrap_err()
                .message
                .contains("budget")
        );
        if mode == 0 {
            assert!(
                checker
                    .candidate_inputs_limited(&reports, Span::default(), limit - 1, scratch)
                    .unwrap_err()
                    .message
                    .contains("budget")
            );
        }
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result = run(&mut checker, MAX_EDGES);
            if short == 0 {
                assert_eq!(result.unwrap(), left);
            } else {
                assert!(result.unwrap_err().message.contains("budget"));
            }
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn seeded_dispatch_candidate_graphs_requalify_mixed_initializer_cycles() {
    let (mut checker, mut reports) = checked("r:3.{->$;->tag:true};alias:((r));copy:{->alias}");
    let alias = checker.local_reads.values().last().unwrap().local;
    let id = *checker
        .local_reads
        .iter()
        .find(|(_, read)| !reports.receivers.contains_key(&read.local))
        .unwrap()
        .0;
    let read = checker.local_reads.get_mut(&id).unwrap();
    read.local = alias;
    read.storage = alias;
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    *local = alias;
    *storage = alias;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let errors = [
        checker
            .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .err()
            .unwrap(),
        checker
            .candidate_graph(&reports, Span::default())
            .err()
            .unwrap(),
        checker
            .direct_graph(&reports, Span::default(), MAX_EDGES)
            .err()
            .unwrap(),
    ];
    assert!(
        errors
            .iter()
            .all(|error| error.message.contains("identity"))
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
