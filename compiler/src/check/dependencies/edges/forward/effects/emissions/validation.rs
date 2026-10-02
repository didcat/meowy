use super::{super::tests::checked, *};

#[test]
pub(crate) fn emission_validation_keeps_direct_composed_alias_and_outer_targets() {
    for source in [
        "row:'out{->name:=1;{'out->2};name=3}",
        "source:{->7;->a:1;->b:true};copy:{->source}",
        "x:1;source:{->view:&x};copy<{view<&int32>}>:'out{{'out->{->source}}}",
        "flag:=false;row:'out{|flag|{'out->value:=1};|!flag|{'out->value:=2}}",
        "f<int32>:(x<int32>){->x};row:{->f(1)}",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        assert!(checker.locals.is_empty());
        let ops: Vec<_> = checker
            .emissions
            .iter()
            .map(|(&id, op)| (id, op.owner, op.targets.clone()))
            .collect();
        for (id, owner, targets) in ops {
            for (index, target) in targets.iter().enumerate() {
                assert_eq!(
                    checker
                        .emission_effect_stage(
                            &reports,
                            owner,
                            Port::Emission(target.id),
                            Span::default()
                        )
                        .unwrap(),
                    Some((id, Some(index)))
                );
            }
            assert_eq!(
                checker
                    .emission_effect_stage(&reports, owner, Port::Normal(id), Span::default())
                    .unwrap(),
                Some((id, None))
            );
        }
    }
}

#[test]
pub(crate) fn emission_validation_keeps_inherited_matcher_lifetime_sites() {
    for source in ["flag:false;|flag|->1", "a:false;b:false;|a| |b| ->1"] {
        crate::compile(source).unwrap();
        let (checker, _) = checked(source, false);
        let (&id, _) = checker.emissions.first_key_value().unwrap();
        let point = &checker.points[id];
        assert_ne!(checker.sites[&point.site.unwrap()].point, Some(id));
    }
    for fault in 0..6 {
        let (mut checker, reports) = checked("flag:false;|flag|->1", false);
        let (&id, op) = checker.emissions.first_key_value().unwrap();
        let target = op.targets[0].id;
        let site = checker.points[id].site.unwrap();
        let parent = checker.points[id].parent.unwrap();
        match fault {
            0 => checker.points[id].parent = None,
            1 => checker.points[parent].owner += 1,
            2 => checker.points[parent].site = None,
            3 => checker.points[parent].complete = false,
            4 => checker.sites.get_mut(&site).unwrap().point = Some(id),
            5 => checker.points[parent].parent = Some(parent),
            _ => unreachable!(),
        }
        assert!(
            checker
                .emission_effect_stage(&reports, 0, Port::Emission(target), Span::default())
                .is_err()
        );
    }
}

#[test]
pub(crate) fn emission_validation_rejects_corrupt_roots_targets_aliases_and_edges() {
    for result in [false, true] {
        for fault in 0..29 {
            let (mut checker, reports) = checked("row:{->value:1}", false);
            let (&id, op) = checker.emissions.first_key_value().unwrap();
            let input = op.input;
            let target = op.targets[0].id;
            let alias = op.targets[0].alias.unwrap();
            let site = checker.points[id].site.unwrap();
            let op = checker.emissions.get_mut(&id).unwrap();
            match fault {
                0 => op.owner += 1,
                1 => op.input = id,
                2 => op.span = Span::default(),
                3 => checker.points[id].complete = false,
                4 => checker.points[id].kind = PointKind::Expr,
                5 => checker.points[id].owner += 1,
                6 => checker.points[input].parent = None,
                7 => checker.points[input].block = None,
                8 => checker.points[input].site = None,
                9 => checker.points[input].complete = false,
                10 => checker.sites.get_mut(&site).unwrap().point = None,
                11 => checker.sites.get_mut(&site).unwrap().complete = false,
                12 => op.targets.clear(),
                13 => op.targets[0].block = usize::MAX,
                14 => op.targets[0].id = usize::MAX,
                15 => op.targets[0].projection = Projection::Primary,
                16 => op.targets[0].field = None,
                17 => op.targets[0].field = Some(String::new()),
                18 => op.targets[0].alias = None,
                19 => op.targets[0].storage = None,
                20 => op.targets[0].storage = Some(reports.locals),
                21 => checker.proofs.aliases.get_mut(&alias).unwrap().emission += 1,
                22 => {
                    checker.proofs.emissions.remove(&target);
                }
                23 => {
                    checker.emission_sources.remove(&target);
                }
                24 => {
                    checker.emission_sources.insert(target, (id, 1));
                }
                25 => op.edges.clear(),
                26 => op.edges[1].route = Route::Result,
                27 => {
                    op.edges.pop();
                }
                28 => op.edges.push(op.edges[0]),
                _ => unreachable!(),
            }
            let port = if result {
                Port::Normal(id)
            } else {
                Port::Emission(target)
            };
            assert!(
                checker
                    .emission_effect_stage(&reports, 0, port, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "fault {fault}"
            );
        }
    }
}
