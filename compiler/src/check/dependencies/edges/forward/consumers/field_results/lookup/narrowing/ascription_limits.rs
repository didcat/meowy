use super::*;
use crate::check::dependencies::edges::forward::{
    consumers::tests::checked,
    results::{Sources, inputs::graph::Visit},
};

#[test]
pub(crate) fn ascribed_field_sources_share_exact_hop_work_and_cache_limits() {
    let depth = 16;
    let mut expr = "r.n".to_owned();
    for _ in 0..depth {
        expr = format!("({expr}~<int32>)");
    }
    let (mut checker, reports) = checked(&format!("r:{{->n:1}};s<int32>:{{->{expr}}}"));
    assert_eq!(checker.typed_ops.len(), depth);
    let direct = reports
        .direct_sources
        .values()
        .find(|(_, direct)| direct.source.is_some())
        .unwrap()
        .1;
    let parts = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count()
        + 1;
    let hops = depth * 2 + 3;
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
            .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), hops)
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
pub(crate) fn seeded_ascribed_field_sources_reject_individually_valid_mixed_cycles() {
    for kind in 0..4 {
        let (mut checker, mut reports) = checked("r:{->n:1};s<int32>:{->((r.n~<int32>)~<int32>)}");
        let direct = reports
            .direct_sources
            .values()
            .find(|(_, direct)| direct.source.is_some())
            .unwrap()
            .1;
        let id = *checker.typed_ops.first_key_value().unwrap().0;
        let child = match kind {
            0 => *checker.typed_ops.last_key_value().unwrap().0,
            1 => checker.coercions[&direct.point].input,
            2 => direct.point,
            _ => {
                *checker
                    .narrowings
                    .iter()
                    .find(|(_, op)| op.input == direct.source.unwrap().field)
                    .unwrap()
                    .0
            }
        };
        assert_ne!(id, child);
        let span = checker.points[direct.point].span;
        for (point, next) in [(id, child), (child, id)] {
            checker.points[point].span = span;
            checker.points[next].parent = Some(point);
            let edges = [
                Edge::new(Port::Entry(point), Port::Entry(next), Route::Next),
                Edge::new(Port::Normal(next), Port::Normal(point), Route::Next),
            ];
            if let Some(op) = checker.typed_ops.get_mut(&point) {
                op.input = next;
                op.span = span;
                op.edges = vec![
                    edges[0],
                    Edge::new(Port::Normal(next), Port::Operation(point), Route::Next),
                    Edge::new(Port::Operation(point), Port::Normal(point), Route::Next),
                ];
                let (_, Effect::Typed(op)) = reports.effects.get_mut(&point).unwrap() else {
                    panic!()
                };
                op.input = next;
            } else if let Some(op) = checker.coercions.get_mut(&point) {
                op.input = next;
                op.span = span;
                op.edges = edges.to_vec();
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&point).unwrap() else {
                    panic!()
                };
                op.input = next;
            } else if let Some(group) = checker.group_inputs.get_mut(&point) {
                group.input = next;
                group.span = span;
                checker.region_edges.insert(point, edges);
            } else {
                let op = checker.narrowings.get_mut(&point).unwrap();
                op.input = next;
                op.span = span;
                op.edges = edges.to_vec();
                let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&point).unwrap() else {
                    panic!()
                };
                op.input = next;
            }
        }
        for (point, next) in [(id, child), (child, id)] {
            let actual = if checker.typed_ops.contains_key(&point) {
                checker
                    .unchanged_ascription_input(&reports, point, 0, Span::default())
                    .unwrap()
            } else if checker.coercions.contains_key(&point) {
                checker
                    .forward_coercion_input(&reports, point, 0, Span::default())
                    .unwrap()
            } else if let Some(&group) = checker.group_inputs.get(&point) {
                Some(
                    checker
                        .qualified_group_input(point, 0, group, Span::default())
                        .unwrap(),
                )
            } else {
                checker
                    .unchanged_narrowing_input(&reports, point, 0, Span::default())
                    .unwrap()
            };
            assert_eq!(actual, Some(next));
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        for start in [id, child] {
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
pub(crate) fn ascribed_field_sources_preserve_unknown_empty_and_multiple_histories() {
    for (source, count) in [
        ("s<int32>:{->(({->n:=1}.n~<int32>))}", None),
        ("r<{n<null>}>:{};s<null>:{->((r.n~<null>))}", Some(0)),
        (
            "flag:=false;r:{|flag|->n:1;|!flag|->n:2};s<int32>:{->((r.n~<int32>))}",
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
