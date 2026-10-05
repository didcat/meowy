use super::*;
use crate::check::dependencies::edges::forward::consumers::tests::checked;

#[test]
pub(crate) fn grouped_field_sources_reject_late_corruption_and_conflicts_without_publication() {
    for fault in 0..15 {
        for observed in [false, true] {
            let (mut checker, mut reports) =
                checked("r:{->n:1};a:{->(r.n)};b:{->((r.n))};v<int32>:7;t:1~<int32>");
            let direct = reports
                .direct_sources
                .values()
                .rev()
                .find(|(_, direct)| direct.source.is_some())
                .unwrap()
                .1;
            let id = checker.group_inputs[&direct.point].input;
            let group = checker.group_inputs[&id];
            match fault {
                0 => checker.group_inputs.get_mut(&id).unwrap().owner += 1,
                1 => checker.group_inputs.get_mut(&id).unwrap().span.end += 1,
                2 => checker.group_inputs.get_mut(&id).unwrap().input = usize::MAX,
                3 => checker.points[id].owner += 1,
                4 => checker.points[group.input].parent = None,
                5 => checker.points[group.input].span.end = group.span.end + 1,
                6 => {
                    checker.region_edges.remove(&id);
                }
                7 => checker.region_edges.get_mut(&id).unwrap().swap(0, 1),
                8 => checker.region_edges.get_mut(&id).unwrap()[1].route = Route::Result,
                9 => {
                    checker
                        .fields
                        .insert(id, checker.fields.first_key_value().unwrap().1.clone());
                }
                10 => {
                    checker
                        .narrowings
                        .insert(id, checker.narrowings.first_key_value().unwrap().1.clone());
                }
                11 => {
                    checker
                        .coercions
                        .insert(id, checker.coercions.first_key_value().unwrap().1.clone());
                }
                12 => {
                    checker
                        .local_reads
                        .insert(id, checker.local_reads.first_key_value().unwrap().1.clone());
                }
                13 => {
                    checker
                        .typed_ops
                        .insert(id, checker.typed_ops.first_key_value().unwrap().1.clone());
                }
                14 => {
                    reports
                        .consumers
                        .insert(id, *reports.consumers.first_key_value().unwrap().1);
                }
                _ => unreachable!(),
            }
            if !observed {
                reports.field_results.clear();
            }
            let before = format!(
                "{reports:?}{:?}{:?}{:?}",
                checker.group_inputs,
                checker.region_edges,
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
                    "{reports:?}{:?}{:?}{:?}",
                    checker.group_inputs,
                    checker.region_edges,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}

#[test]
pub(crate) fn seeded_grouped_field_cycles_fail_after_individually_valid_mixed_edges() {
    for mixed in [false, true] {
        let (mut checker, mut reports) = checked("r:{->n:1};s:{->((r.n))}");
        let direct = reports
            .direct_sources
            .values()
            .find(|(_, direct)| direct.source.is_some())
            .unwrap()
            .1;
        let id = direct.point;
        let child = if mixed {
            *checker
                .narrowings
                .iter()
                .find(|(_, op)| op.input == direct.source.unwrap().field)
                .unwrap()
                .0
        } else {
            checker.group_inputs[&id].input
        };
        let span = checker.points[id].span;
        for (point, next) in [(id, child), (child, id)] {
            checker.points[point].span = span;
            checker.points[next].parent = Some(point);
            let edges = [
                Edge::new(Port::Entry(point), Port::Entry(next), Route::Next),
                Edge::new(Port::Normal(next), Port::Normal(point), Route::Next),
            ];
            if mixed && point == child {
                let op = checker.narrowings.get_mut(&point).unwrap();
                op.input = next;
                op.span = span;
                op.edges = edges.to_vec();
                let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&point).unwrap() else {
                    panic!()
                };
                op.input = next;
            } else {
                let group = checker.group_inputs.get_mut(&point).unwrap();
                group.input = next;
                group.span = span;
                checker.region_edges.insert(point, edges);
            }
        }
        let group = checker.group_inputs[&id];
        assert_eq!(
            checker
                .qualified_group_input(id, 0, group, Span::default())
                .unwrap(),
            child
        );
        if mixed {
            assert_eq!(
                checker
                    .unchanged_narrowing_input(&reports, child, 0, Span::default())
                    .unwrap(),
                Some(id)
            );
        } else {
            let group = checker.group_inputs[&child];
            assert_eq!(
                checker
                    .qualified_group_input(child, 0, group, Span::default())
                    .unwrap(),
                id
            );
        }
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
        let before = format!("{reports:?}{:?}", checker.edge_counts());
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
