use super::*;
use crate::check::dependencies::{OperationKind, edges::forward::consumers::tests::checked};

#[test]
pub(crate) fn local_field_sources_reject_read_binding_and_conflicting_metadata_atomically() {
    for fault in 0..42 {
        for observed in [false, true] {
            let (mut checker, mut reports) =
                checked("r:{->n:1};a:r.n;first:{->a};b<int32>:((r.n~<int32>));last:{->((b))}");
            let direct = reports
                .direct_sources
                .values()
                .rev()
                .find(|(_, direct)| direct.source.is_some())
                .unwrap()
                .1;
            let (&id, op) = checker
                .local_reads
                .iter()
                .find(|(id, _)| checker.points[**id].block == checker.points[direct.point].block)
                .unwrap();
            let local = op.local;
            let init = reports.initializers[&local];
            let statement = init.statement;
            let root = init.input.unwrap();
            let (
                owner,
                Effect::Read {
                    local: read_local,
                    storage,
                    normal,
                    control,
                },
            ) = reports.effects.get_mut(&id).unwrap()
            else {
                panic!()
            };
            match fault {
                0 => *owner += 1,
                1 => *read_local = usize::MAX,
                2 => *storage = usize::MAX,
                3 => *normal = false,
                4 => *control = !*control,
                5 => {
                    checker.local_reads.remove(&id);
                }
                6 => checker.local_reads.get_mut(&id).unwrap().owner += 1,
                7 => checker.local_reads.get_mut(&id).unwrap().edges.swap(0, 1),
                8 => checker.points[id].kind = PointKind::Stmt,
                9 => checker.points[id].complete = false,
                10 => checker.points[id].block = None,
                11 => checker.points[id].span.end += 1,
                12 => checker.local_reads.get_mut(&id).unwrap().storage = usize::MAX,
                13 => {
                    reports.index.operations.remove(&id);
                }
                14 => {
                    *reports.index.operations.get_mut(&id).unwrap() += 1;
                }
                15 => reports.initializers.get_mut(&local).unwrap().owner += 1,
                16 => reports.initializers.get_mut(&local).unwrap().statement = id,
                17 => reports.initializers.get_mut(&local).unwrap().input = None,
                18 => {
                    reports.effects.remove(&statement);
                }
                19 => checker.operations.get_mut(&statement).unwrap().input = None,
                20 => checker
                    .operations
                    .get_mut(&statement)
                    .unwrap()
                    .edges
                    .clear(),
                21 => checker.points[statement].complete = false,
                22 => checker.points[statement].kind = PointKind::Expr,
                23 => checker.points[statement].site = None,
                24 => checker.points[root].parent = Some(id),
                25 => checker.points[root].block = None,
                26 => checker.points[root].site = None,
                27 => {
                    reports.entries.remove(&0);
                }
                28 => {
                    checker.proofs.bindings.remove(&local);
                }
                29 => checker.points[root].owner += 1,
                30 => reports.initializers.get_mut(&local).unwrap().input = Some(usize::MAX),
                31 => {
                    reports.index.operations.remove(&statement);
                }
                32 => reports.effects.get_mut(&statement).unwrap().0 += 1,
                33 => {
                    let (_, Effect::Storage { kind, .. }) =
                        reports.effects.get_mut(&statement).unwrap()
                    else {
                        panic!()
                    };
                    *kind = OperationKind::Write;
                }
                34 => {
                    let (_, Effect::Storage { control, .. }) =
                        reports.effects.get_mut(&statement).unwrap()
                    else {
                        panic!()
                    };
                    *control = !*control;
                }
                35 => checker.points[statement].span.end += 1,
                36 => {
                    checker
                        .fields
                        .insert(id, checker.fields.first_key_value().unwrap().1.clone());
                }
                37 => {
                    checker
                        .narrowings
                        .insert(id, checker.narrowings.first_key_value().unwrap().1.clone());
                }
                38 => {
                    checker
                        .group_inputs
                        .insert(id, *checker.group_inputs.first_key_value().unwrap().1);
                }
                39 => {
                    checker
                        .coercions
                        .insert(id, checker.coercions.first_key_value().unwrap().1.clone());
                }
                40 => {
                    checker
                        .typed_ops
                        .insert(id, checker.typed_ops.first_key_value().unwrap().1.clone());
                }
                41 => {
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
                checker.local_reads,
                checker.operations,
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
                    checker.local_reads,
                    checker.operations,
                    checker.edge_counts()
                ),
                before
            );
        }
    }
}
