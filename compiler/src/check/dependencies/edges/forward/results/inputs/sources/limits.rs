use super::{super::super::tests::checked, *};

pub(super) const SOURCE: &str = "r:{->7;->a:1;->b:true};s:{->((r))}";

#[test]
pub(crate) fn candidate_sources_reject_late_link_and_source_corruption_atomically() {
    for fault in 0..10 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&key, &(_, input)) = reports
            .candidate_inputs
            .iter()
            .rev()
            .find(|(_, (_, input))| input.source.is_some())
            .unwrap();
        let port = Port::Emission(input.candidate.emission);
        let slot = input.source.unwrap();
        let anchor = reports.results[&slot.block].1.consumer.unwrap();
        match fault {
            0 => reports.slot_uses.get_mut(&port).unwrap().0 += 1,
            1 => reports.slot_uses.get_mut(&port).unwrap().1.block = usize::MAX,
            2 => reports.slot_uses.get_mut(&port).unwrap().1.block = key.0,
            3 => reports.slot_uses.get_mut(&port).unwrap().1.index += 1,
            4 => reports.slot_uses.get_mut(&port).unwrap().1.index = usize::MAX,
            5 => {
                reports.consumers.remove(&anchor);
            }
            6 => reports.results.get_mut(&slot.block).unwrap().1.consumer = None,
            7 => reports.blocks.get_mut(&slot.block).unwrap().1.result = false,
            8 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
            9 => reports.consumers.get_mut(&anchor).unwrap().0 += 1,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}{:?}", checker.bodies, checker.edge_counts());
        let error = checker
            .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap_err();
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(
            format!("{reports:?}{:?}{:?}", checker.bodies, checker.edge_counts()),
            before
        );
    }
}

#[test]
pub(crate) fn candidate_sources_reject_links_for_direct_values_and_opaque_projections() {
    for source in ["r:{->n:1}", "f<{n<int32>}>:(){->n:1};s:{->f()}"] {
        let (mut checker, mut reports) = checked(source);
        let (&(block, slot, _), &(owner, input)) =
            reports.candidate_inputs.last_key_value().unwrap();
        reports.slot_uses.insert(
            Port::Emission(input.candidate.emission),
            (owner, Slot { block, index: slot }),
        );
        let before = format!("{reports:?}");
        let error = checker
            .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap_err();
        assert!(error.message.contains("candidate-source identity"));
        assert_eq!(format!("{reports:?}"), before);
    }
}

#[test]
pub(crate) fn candidate_sources_bound_source_cache_room_and_reuse() {
    let (mut checker, reports) = checked(SOURCE);
    let keys: Vec<_> = reports
        .candidate_inputs
        .iter()
        .filter(|(_, (_, input))| input.source.is_some())
        .map(|(&key, _)| key)
        .collect();
    let mut ctx = Context::new(&reports, 3);
    for &key in &keys {
        assert_eq!(
            checker
                .candidate_input(&mut ctx, key, Span::default())
                .unwrap(),
            reports.candidate_inputs[&key]
        );
    }
    assert_eq!(ctx.parts, 0);
    assert_eq!(ctx.sources.len(), 1);
    let mut ctx = Context::new(&reports, 2);
    let error = checker
        .candidate_input(&mut ctx, keys[0], Span::default())
        .unwrap_err();
    assert!(error.message.contains("candidate-source budget"));
    assert!(ctx.sources.is_empty());
}

#[test]
pub(crate) fn candidate_sources_resolve_deep_chains_once_for_wide_compositions() {
    let mut costs = [[0; 2]; 2];
    for (row, width) in [1, 8].into_iter().enumerate() {
        for (column, depth) in [0, 16].into_iter().enumerate() {
            let fields = (0..width)
                .map(|part| format!(";->n{part}:1"))
                .collect::<String>();
            let source = format!(
                "v:{{->{}{{->7{fields}}}{}}}",
                "(".repeat(depth),
                ")".repeat(depth)
            );
            let (mut checker, reports) = checked(&source);
            let before = checker.flow.work;
            let (inputs, _) = checker
                .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap();
            costs[row][column] = checker.flow.work - before;
            assert_eq!(inputs, reports.candidate_inputs);
            assert_eq!(
                inputs
                    .values()
                    .filter(|(_, input)| input.source.is_some())
                    .count(),
                width + 1
            );
        }
    }
    let shallow = costs[0][1] - costs[0][0];
    let wide = costs[1][1] - costs[1][0];
    assert!(shallow > 0);
    assert_eq!(wide, shallow);
}
