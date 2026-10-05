use super::*;
use crate::check::dependencies::{
    CoercionKind,
    edges::forward::{consumers::tests::checked, results::inputs::graph::Visit},
};

#[test]
pub(crate) fn forward_field_sources_require_observed_results_at_every_coercion() {
    for depth in 0..3 {
        for missing in [false, true] {
            let (mut checker, mut reports) = checked("r:{->n:1};s<int32>:{->((r.n))}");
            let (&key, &(_, direct)) = reports
                .direct_sources
                .iter()
                .find(|(_, (_, direct))| direct.source.is_some())
                .unwrap();
            let mut id = direct.point;
            for _ in 0..depth {
                id = checker.group_inputs[&checker.coercions[&id].input].input;
            }
            if missing {
                reports.effects.remove(&id);
            } else {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            let sources = checker.direct_sources(&reports, Span::default()).unwrap().0;
            assert_eq!(sources[&key].1.point, direct.point);
            assert_eq!(sources[&key].1.source, None);
            assert!(
                checker
                    .direct_walk_report(&reports, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
            reports.direct_sources = sources;
            let walk = checker
                .direct_walk_report(&reports, Span::default())
                .unwrap()
                .0;
            assert!(
                walk.visits
                    .contains(&Visit::Value(key, reports.candidate_inputs[&key].1))
            );
            assert!(
                !walk
                    .visits
                    .iter()
                    .any(|visit| matches!(visit, Visit::Field(_, _, _)))
            );
        }
    }
}

#[test]
pub(crate) fn forward_field_sources_keep_conversion_projection_stops_and_other_wrappers_opaque() {
    for source in [
        "r:{->n:1};s<int32><null>:{->((r.n))}",
        "r:{->n:{->1;->tag:true}};s<int32>:{->((r.n))}",
        "d:@\"debug\";s<int32>:{->((d.panic(\"stop\")))}",
        "r:{->n:1};s<int32>:{->((r.n~<int32>))}",
        "r:{->n:1};n:r.n;s<int32>:{->((n))}",
        "f<int32>:(){->1};s<int32>:{->((f()))}",
        "r:{->n:1};p:&r;s<int32>:{->((p.n))}",
        "r:{->n:1};p:&r;s<int32>:{->(((*p).n))}",
        "r<{n<int32><null>}>:{->n:1};|r.n<int32>|s<int32>:{->((r.n))}",
    ] {
        let (mut checker, reports) = checked(source);
        assert!(
            reports
                .direct_sources
                .values()
                .all(|(_, direct)| direct.source.is_none()),
            "{source}"
        );
        assert!(
            !reports
                .expanded_walk
                .visits
                .iter()
                .any(|visit| matches!(visit, Visit::Field(_, _, _))),
            "{source}"
        );
        let ids: Vec<_> = checker
            .coercions
            .iter()
            .filter(|(_, op)| op.kind != CoercionKind::Forward || op.primary)
            .map(|(&id, op)| (id, op.owner))
            .collect();
        for (id, owner) in ids {
            assert_eq!(
                checker
                    .field_narrowing_source(
                        &mut Lookup::new(&reports, MAX_EDGES),
                        id,
                        owner,
                        Span::default(),
                        MAX_GROUPS
                    )
                    .unwrap(),
                None,
                "{source}"
            );
        }
    }
}

#[test]
pub(crate) fn forward_field_sources_reject_late_corruption_and_producer_conflicts_atomically() {
    for fault in 0..25 {
        for observed in [false, true] {
            let (mut checker, mut reports) =
                checked("r:{->n:1};a<int32>:{->(r.n)};b<int32>:{->((r.n))};t:1~<int32>");
            let direct = reports
                .direct_sources
                .values()
                .rev()
                .find(|(_, direct)| direct.source.is_some())
                .unwrap()
                .1;
            let id = checker.group_inputs[&checker.coercions[&direct.point].input].input;
            let input = checker.coercions[&id].input;
            let (owner, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            match fault {
                0 => *owner += 1,
                1 => op.input = id,
                2 => op.op = CoercionKind::Convert,
                3 => op.primary = true,
                4 => op.control = !op.control,
                5 => op.projected = true,
                6 => op.operation = true,
                7 => checker.coercions.get_mut(&id).unwrap().owner += 1,
                8 => checker.coercions.get_mut(&id).unwrap().span.end += 1,
                9 => checker.coercions.get_mut(&id).unwrap().edges[1].route = Route::Result,
                10 => checker.coercions.get_mut(&id).unwrap().edges.swap(0, 1),
                11 => checker.points[input].parent = None,
                12 => checker.points[input].span.end += 1,
                13 => checker.points[id].complete = false,
                14 => checker.points[id].owner += 1,
                15 => checker.points[id].block = None,
                16 => {
                    checker.coercions.remove(&id);
                }
                17 => {
                    checker
                        .fields
                        .insert(id, checker.fields.first_key_value().unwrap().1.clone());
                }
                18 => {
                    checker
                        .narrowings
                        .insert(id, checker.narrowings.first_key_value().unwrap().1.clone());
                }
                19 => {
                    checker
                        .group_inputs
                        .insert(id, *checker.group_inputs.first_key_value().unwrap().1);
                }
                20 => {
                    checker
                        .local_reads
                        .insert(id, checker.local_reads.first_key_value().unwrap().1.clone());
                }
                21 => {
                    checker
                        .typed_ops
                        .insert(id, checker.typed_ops.first_key_value().unwrap().1.clone());
                }
                22 => {
                    reports
                        .consumers
                        .insert(id, *reports.consumers.first_key_value().unwrap().1);
                }
                23 => checker.points[input].owner += 1,
                24 => checker.points[input].block = None,
                _ => unreachable!(),
            }
            if !observed {
                reports.field_results.clear();
            }
            let before = format!(
                "{reports:?}{:?}{:?}",
                checker.coercions,
                checker.edge_counts()
            );
            let error = checker
                .direct_sources(&reports, Span::default())
                .unwrap_err();
            assert!(
                error.message.contains("identity"),
                "{fault}, {observed}: {error:?}"
            );
            assert!(
                checker
                    .direct_graph(&reports, Span::default(), MAX_EDGES)
                    .err()
                    .unwrap()
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
            assert_eq!(
                format!(
                    "{reports:?}{:?}{:?}",
                    checker.coercions,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}
