use super::*;
use crate::check::dependencies::edges::forward::{
    consumers::tests::checked,
    results::{Sources, inputs::graph::Visit},
};

#[test]
pub(crate) fn local_field_sources_share_exact_mixed_hops_work_and_cached_payload() {
    let depth = 16;
    let mut source = "r:{->n:1};n0:r.n;".to_owned();
    for index in 1..=depth {
        source.push_str(&format!("n{index}<int32>:((n{}~<int32>));", index - 1));
    }
    source.push_str(&format!("s:{{->n{depth};->again:(n{depth})}}"));
    let (mut checker, reports) = checked(&source);
    let sources: Vec<_> = reports
        .direct_sources
        .values()
        .filter(|(_, direct)| direct.source.is_some())
        .map(|(_, direct)| *direct)
        .collect();
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].source, sources[1].source);
    let direct = sources[0];
    let parts = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count()
        + 1;
    let hops = depth * 8 + 3;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (input, limit, parts, accepted) in [
        (direct.point, hops, parts, true),
        (direct.point, hops - 1, parts, false),
        (direct.point, 0, parts, false),
        (direct.source.unwrap().field, 0, parts, true),
        (direct.point, hops, parts - 1, false),
    ] {
        let mut ctx = Lookup::new(&reports, parts);
        let result = checker.field_narrowing_source(&mut ctx, input, 0, Span::default(), limit);
        if accepted {
            assert_eq!(result.unwrap(), direct.source);
            assert_eq!(ctx.parts, 0);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    let mut ctx = Lookup::new(&reports, parts);
    let work = checker.flow.work;
    assert_eq!(
        checker
            .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), hops)
            .unwrap(),
        direct.source
    );
    let work = checker.flow.work - work;
    assert_eq!(ctx.parts, 0);
    assert_eq!(
        checker
            .field_narrowing_source(&mut ctx, sources[1].point, 0, Span::default(), hops + 1)
            .unwrap(),
        direct.source
    );
    assert_eq!(ctx.parts, 0);
    assert!(
        checker
            .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), hops - 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.field_narrowing_source(
            &mut Lookup::new(&reports, parts),
            direct.point,
            0,
            Span::default(),
            hops,
        );
        if short == 0 {
            assert_eq!(result.unwrap(), direct.source);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn seeded_local_field_sources_reject_cycles_across_qualified_initializer_jumps() {
    for source in [
        "r:{->n:1};n:r.n;copy:n;s:{->copy}",
        "r:{->n:1};n:r.n;copy<int32>:((n~<int32>));s:{->((copy))}",
    ] {
        let (mut checker, mut reports) = checked(source);
        let direct = reports
            .direct_sources
            .values()
            .find(|(_, direct)| direct.source.is_some())
            .unwrap()
            .1;
        let (&last, op) = checker
            .local_reads
            .iter()
            .find(|(id, _)| checker.points[**id].block == checker.points[direct.point].block)
            .unwrap();
        let local = op.local;
        let init = reports.initializers[&local];
        let root = init.input.unwrap();
        let id = *checker
            .local_reads
            .keys()
            .find(|id| checker.points[**id].site == checker.points[init.statement].site)
            .unwrap();
        let op = checker.local_reads.get_mut(&id).unwrap();
        op.local = local;
        op.storage = local;
        let (
            _,
            Effect::Read {
                local: read_local,
                storage,
                ..
            },
        ) = reports.effects.get_mut(&id).unwrap()
        else {
            panic!()
        };
        *read_local = local;
        *storage = local;
        for read in [id, last] {
            assert_eq!(
                checker
                    .read_initializer_input(&reports, read, 0, Span::default())
                    .unwrap(),
                Some(root)
            );
            assert_ne!(checker.points[root].parent, Some(read));
        }
        let mut point = root;
        while point != id {
            point = if let Some(&group) = checker.group_inputs.get(&point) {
                checker
                    .qualified_group_input(point, 0, group, Span::default())
                    .unwrap()
            } else if checker.coercions.contains_key(&point) {
                checker
                    .forward_coercion_input(&reports, point, 0, Span::default())
                    .unwrap()
                    .unwrap()
            } else if checker.typed_ops.contains_key(&point) {
                checker
                    .unchanged_ascription_input(&reports, point, 0, Span::default())
                    .unwrap()
                    .unwrap()
            } else {
                checker
                    .unchanged_narrowing_input(&reports, point, 0, Span::default())
                    .unwrap()
                    .unwrap()
            };
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        for start in [id, root, direct.point] {
            let error = checker
                .field_narrowing_source(
                    &mut Lookup::new(&reports, MAX_EDGES),
                    start,
                    0,
                    Span::default(),
                    MAX_GROUPS,
                )
                .unwrap_err();
            assert!(error.message.contains("field-narrowing identity"));
        }
        assert!(
            checker
                .direct_sources(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn local_field_sources_preserve_unknown_empty_and_multiple_histories() {
    for (source, count) in [
        ("n:{->n:=1}.n;copy:n;s:{->copy}", None),
        (
            "r<{n<null>}>:{};n<null>:r.n;copy:n;s<null>:{->copy}",
            Some(0),
        ),
        (
            "flag:=false;r:{|flag|->n:1;|!flag|->n:2};n:r.n;copy:n;s:{->copy}",
            Some(2),
        ),
    ] {
        let (_, reports) = checked(source);
        let sources: Vec<_> = reports
            .direct_sources
            .values()
            .filter_map(|(_, direct)| direct.source)
            .collect();
        assert_eq!(sources.len(), 1, "{source}");
        let slot = sources[0].slot;
        let history = &reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index];
        if let Some(count) = count {
            let Sources::Candidates(values) = history else {
                panic!()
            };
            assert_eq!(values.len(), count);
        } else {
            assert_eq!(*history, Sources::Unknown);
        }
    }
}
