use super::*;
use crate::check::dependencies::{
    TypedKind,
    edges::forward::{consumers::tests::checked, results::inputs::graph::Visit},
};

#[test]
pub(crate) fn ascribed_field_sources_require_results_independently_of_operation_visits() {
    for depth in 0..2 {
        for (result, operation, missing) in [
            (true, true, false),
            (true, false, false),
            (false, true, false),
            (false, false, false),
            (true, true, true),
        ] {
            let (mut checker, mut reports) =
                checked("r:{->n:1};s<int32>:{->((r.n~<int32>)~<int32>)}");
            let (&key, &(_, direct)) = reports
                .direct_sources
                .iter()
                .find(|(_, (_, direct))| direct.source.is_some())
                .unwrap();
            assert_eq!(checker.typed_ops.len(), 2);
            let id = *checker.typed_ops.keys().nth(depth).unwrap();
            if missing {
                reports.effects.remove(&id);
            } else {
                let (_, Effect::Typed(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.result = result;
                op.operation = operation;
            }
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            let sources = checker.direct_sources(&reports, Span::default()).unwrap().0;
            let allowed = result && !missing;
            assert_eq!(sources[&key].1.point, direct.point);
            assert_eq!(
                sources[&key].1.source,
                if allowed { direct.source } else { None }
            );
            let walk = checker.direct_walk_report(&reports, Span::default());
            if allowed {
                assert_eq!(walk.unwrap().0, reports.expanded_walk);
            } else {
                assert!(walk.unwrap_err().message.contains("identity"));
            }
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
            reports.direct_sources = sources;
            let walk = checker
                .direct_walk_report(&reports, Span::default())
                .unwrap()
                .0;
            let input = reports.candidate_inputs[&key].1;
            let visit = if allowed {
                Visit::Field(key, input, direct.source.unwrap())
            } else {
                Visit::Value(key, input)
            };
            assert!(walk.visits.contains(&visit));
        }
    }
}

#[test]
pub(crate) fn ascribed_field_sources_preserve_predicate_changed_stopped_and_other_boundaries() {
    for source in [
        "r:{->n:1};s:{->((r.n<int32>))}",
        "<U>:<int32><null>;r:{->n:1};s:{->((r.n~<U>))}",
        "d:@\"debug\";s<int32>:{->((d.panic(\"stop\")~<int32>))}",
        "r:{->n:1};n:r.n;s<int32>:{->((n~<int32>))}",
        "f<int32>:(){->1};s<int32>:{->((f()~<int32>))}",
        "r:{->n:1};p:&r;s<int32>:{->((p.n~<int32>))}",
        "r:{->n:1};p:&r;s<int32>:{->(((*p).n~<int32>))}",
        "r<{n<int32><null>}>:{->n:1};|r.n<int32>|s<int32>:{->((r.n~<int32>))}",
        "r:{->n:{->1;->tag:true}};s<int32>:{->((r.n~<{-><int32>;tag<boolean>}>))}",
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
            .typed_ops
            .iter()
            .filter(|(_, op)| op.kind != TypedKind::Ascription || op.changed || !op.normal)
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
pub(crate) fn ascribed_field_sources_reject_late_corruption_and_conflicts_atomically() {
    for fault in 0..27 {
        for observed in [false, true] {
            let (mut checker, mut reports) =
                checked("r:{->n:1};a<int32>:{->(r.n~<int32>)};b<int32>:{->((r.n~<int32>))}");
            let id = *checker.typed_ops.last_key_value().unwrap().0;
            let input = checker.typed_ops[&id].input;
            let (owner, Effect::Typed(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            match fault {
                0 => *owner += 1,
                1 => op.input = id,
                2 => op.op = TypedKind::Predicate,
                3 => op.changed = true,
                4 => op.normal = false,
                5 => op.control = !op.control,
                6 => checker.typed_ops.get_mut(&id).unwrap().owner += 1,
                7 => checker.typed_ops.get_mut(&id).unwrap().span.end += 1,
                8 => checker.typed_ops.get_mut(&id).unwrap().edges[1].route = Route::Result,
                9 => checker.typed_ops.get_mut(&id).unwrap().edges.swap(0, 1),
                10 => checker.points[id].complete = false,
                11 => checker.points[id].owner += 1,
                12 => checker.points[id].block = None,
                13 => checker.points[input].parent = None,
                14 => checker.points[input].owner += 1,
                15 => checker.points[input].block = None,
                16 => checker.typed_ops.get_mut(&id).unwrap().input = id,
                17 => {
                    reports.index.operations.remove(&id);
                }
                18 => {
                    *reports.index.operations.get_mut(&id).unwrap() += 1;
                }
                19 => {
                    checker
                        .fields
                        .insert(id, checker.fields.first_key_value().unwrap().1.clone());
                }
                20 => {
                    checker
                        .narrowings
                        .insert(id, checker.narrowings.first_key_value().unwrap().1.clone());
                }
                21 => {
                    checker
                        .group_inputs
                        .insert(id, *checker.group_inputs.first_key_value().unwrap().1);
                }
                22 => {
                    checker
                        .coercions
                        .insert(id, checker.coercions.first_key_value().unwrap().1.clone());
                }
                23 => {
                    checker
                        .local_reads
                        .insert(id, checker.local_reads.first_key_value().unwrap().1.clone());
                }
                24 => {
                    reports
                        .consumers
                        .insert(id, *reports.consumers.first_key_value().unwrap().1);
                }
                25 => {
                    checker.typed_ops.remove(&id);
                }
                26 => checker.points[input].complete = false,
                _ => unreachable!(),
            }
            if !observed {
                reports.field_results.clear();
            }
            let before = format!(
                "{reports:?}{:?}{:?}",
                checker.typed_ops,
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
                    checker.typed_ops,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}
