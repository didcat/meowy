use super::{super::tests::checked, *};

#[test]
pub(crate) fn reborrow_effect_validation_keeps_explicit_and_implicit_modes_and_roots() {
    for source in [
        "n:1;p:&n;q:&*p",
        "n:=1;p:&!n;q:&*p",
        "n:=1;p:&!n;q:&!*p",
        "n:=1;p:&!n;q<&int32>:p",
        "n:=1;p:&!n;q<&int32>:&!*p",
        "id<&!int32>:(p<&!int32>){->p};n:=1;q<&int32>:id(&!n)",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let ids = checker
            .reborrow_ops
            .iter()
            .map(|(&id, op)| (id, op.owner))
            .collect::<Vec<_>>();
        assert!(!ids.is_empty());
        for (id, owner) in ids {
            for port in [Port::Operation(id), Port::Normal(id)] {
                assert!(
                    checker
                        .validate_reborrow(&reports, owner, port, Span::default())
                        .unwrap()
                );
            }
            assert!(
                !checker
                    .validate_reborrow(&reports, owner, Port::Entry(id), Span::default())
                    .unwrap()
            );
        }
    }
}

#[test]
pub(crate) fn reborrow_effect_validation_rejects_corrupt_modes_sites_roots_and_edges() {
    for source in ["n:=1;p:&!n;q:&!*p", "n:=1;p:&!n;q<&int32>:p"] {
        for normal in [false, true] {
            for fault in 0..30 {
                let (mut checker, mut reports) = checked(source, false);
                let (&id, op) = checker.reborrow_ops.first_key_value().unwrap();
                let parent = op.parent;
                reports.entries.get_mut(&0).unwrap().1.ports = vec![if normal {
                    Port::Normal(id)
                } else {
                    Port::Operation(id)
                }];
                let before = reports.effects.clone();
                let op = checker.reborrow_ops.get_mut(&id).unwrap();
                match fault {
                    0 => op.owner += 1,
                    1 => checker.points[id].owner += 1,
                    2 => checker.points[id].complete = false,
                    3 => checker.points[id].kind = PointKind::Read,
                    4 => checker.points[id].span = Span::default(),
                    5 => op.span = Span::default(),
                    6 => op.parent = usize::MAX,
                    7 => op.parent = id,
                    8 => checker.points[parent].parent = None,
                    9 => checker.points[parent].owner += 1,
                    10 => checker.points[parent].block = None,
                    11 => checker.points[parent].complete = false,
                    12 => checker.points[parent].kind = PointKind::Stmt,
                    13 => op.site = None,
                    14 => op.parent_mode = None,
                    15 => op.site = Some(checker.reborrows),
                    16 => {
                        op.mode = ReferenceMode::Exclusive;
                        op.parent_mode = Some(ReferenceMode::Shared);
                    }
                    17 => checker.reborrows = 0,
                    18 => op.edges.clear(),
                    19 => op.edges[0].from = Port::Normal(id),
                    20 => op.edges[0].to = Port::Normal(parent),
                    21 => op.edges[1].from = Port::Entry(parent),
                    22 => op.edges[1].to = Port::Normal(id),
                    23 => op.edges[2].to = Port::Normal(parent),
                    24 => op.edges[2].route = Route::Returned,
                    25 => op.edges.push(op.edges[2]),
                    26 => op.edges.swap(0, 1),
                    27 => {
                        reports.index.operations.remove(&id);
                    }
                    28 => {
                        reports.index.operations.insert(id, 9);
                    }
                    29 => checker.points.truncate(id),
                    _ => unreachable!(),
                }
                let ops = checker.reborrow_ops.clone();
                let counts = checker.edge_counts();
                let error = checker
                    .operation_effects(&reports, Span::default())
                    .unwrap_err();
                assert_eq!(
                    error.code, "B001",
                    "{source}, fault {fault}, normal {normal}"
                );
                assert!(error.message.contains("reborrow-effect identity"));
                assert_eq!(reports.effects, before);
                assert_eq!(checker.reborrow_ops, ops);
                assert_eq!(checker.edge_counts(), counts);
            }
        }
    }
}
