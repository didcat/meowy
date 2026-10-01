use super::{super::tests::checked, *};

#[test]
pub(crate) fn element_effects_reject_corrupt_owned_source_bounds_and_storage() {
    for normal in [false, true] {
        for fault in 0..8 {
            let (mut checker, mut reports) = checked("r:{->xs:[1]};p:&(r.xs[1])", false);
            let id = *checker.elements.first_key_value().unwrap().0;
            reports.entries.get_mut(&0).unwrap().1.ports = vec![if normal {
                Port::Normal(id)
            } else {
                Port::Address { point: id, step: 0 }
            }];
            let ElementSource::Place {
                place,
                storage,
                counts,
            } = &mut checker.elements.get_mut(&id).unwrap().source
            else {
                panic!()
            };
            match fault {
                0 => place.root = reports.locals,
                1 => *storage = reports.locals,
                2 => *storage += 1,
                3 => counts.clear(),
                4 => counts.push(1),
                5 => place.fields[0] = counts[0],
                6 => counts[0] = 0,
                7 => place.fields = vec![0; crate::list::MAX_WRITE_PATH + 1],
                _ => unreachable!(),
            }
            let before = reports.effects.clone();
            let ops = checker.elements.clone();
            let edges = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert!(
                error
                    .message
                    .contains(if fault == 7 { "budget" } else { "identity" })
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.elements, ops);
            assert_eq!(checker.edge_counts(), edges);
        }
    }
}

#[test]
pub(crate) fn element_effects_reject_corrupt_temporary_cells_and_statement_sites() {
    for normal in [false, true] {
        for fault in 0..17 {
            let (mut checker, mut reports) = checked("x:*(&([1][1]))", false);
            let (&id, op) = checker.elements.first_key_value().unwrap();
            let parent = op.parent;
            let ElementSource::Temporary { local, statement } = op.source else {
                panic!()
            };
            let root = checker.sites[&statement].point.unwrap();
            reports.entries.get_mut(&0).unwrap().1.ports = vec![if normal {
                Port::Normal(id)
            } else {
                Port::Address { point: id, step: 0 }
            }];
            match fault {
                0 => {
                    checker.elements.get_mut(&id).unwrap().source = ElementSource::Temporary {
                        local: reports.locals,
                        statement,
                    }
                }
                1 => {
                    checker.elements.get_mut(&id).unwrap().source = ElementSource::Temporary {
                        local,
                        statement: checker.statements,
                    }
                }
                2 => {
                    checker.proofs.temporaries.remove(&local);
                }
                3 => {
                    checker.proofs.temporaries.insert(local, usize::MAX);
                }
                4 => checker.points[id].site = None,
                5 => checker.points[parent].site = None,
                6 => {
                    checker.sites.remove(&statement);
                }
                7 => checker.sites.get_mut(&statement).unwrap().owner += 1,
                8 => checker.sites.get_mut(&statement).unwrap().complete = false,
                9 => checker.sites.get_mut(&statement).unwrap().block = None,
                10 => checker.sites.get_mut(&statement).unwrap().point = None,
                11 => checker.points[root].complete = false,
                12 => checker.points[root].kind = PointKind::Expr,
                13 => checker.points[root].owner += 1,
                14 => checker.points[root].site = None,
                15 => checker.points[root].block = None,
                16 => checker.points[root].span = Span::default(),
                _ => unreachable!(),
            }
            let before = reports.effects.clone();
            let ops = checker.elements.clone();
            let edges = checker.edge_counts();
            let error = checker
                .operation_effects(&reports, Span::default())
                .unwrap_err();
            assert!(
                error.message.contains("element-effect identity"),
                "fault {fault}, normal {normal}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.elements, ops);
            assert_eq!(checker.edge_counts(), edges);
        }
    }
}

#[test]
pub(crate) fn element_effects_keep_canonical_alias_sources_and_reject_changed_roots() {
    let source = "f:(flag<boolean>)'out{|flag|{'out->xs:=[1,2];p:&(xs[1])};|!flag|{'out->xs:=[3,4];p:&(xs[2])}}";
    crate::compile(source).unwrap();
    let (mut checker, mut reports) = checked(source, false);
    let mut stores = Vec::new();
    let mut roots = Vec::new();
    for (&id, op) in &checker.elements {
        let (_, Effect::Element(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(observed.source, op.source);
        let ElementSource::Place { place, storage, .. } = &observed.source else {
            panic!()
        };
        roots.push(place.root);
        stores.push(*storage);
    }
    assert_eq!(stores.len(), 2);
    assert_ne!(roots[0], roots[1]);
    assert_eq!(stores[0], stores[1]);
    let (&id, op) = checker.elements.first_key_value().unwrap();
    let owner = op.owner;
    reports.entries.get_mut(&owner).unwrap().1.ports = vec![Port::Address { point: id, step: 0 }];
    checker.proofs.aliases.get_mut(&roots[0]).unwrap().root = reports.locals;
    let before = reports.effects.clone();
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("element-effect identity")
    );
    assert_eq!(reports.effects, before);
}
