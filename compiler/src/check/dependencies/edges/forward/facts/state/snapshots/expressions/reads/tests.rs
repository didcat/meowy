use super::super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_expression_reads_use_declared_parameter_values_and_original_versions() {
    let (mut checker, program, _, snapshots) =
        capture("f:(n<uint8>){n};g:(b<boolean><null>){b};f(7);g(null)");
    for (&point, read) in &checker.local_reads {
        let fact = &snapshots.expressions[&point];
        let value = fact.value.as_ref().unwrap();
        assert_eq!(fact.owner, read.owner);
        assert_eq!(
            Some(&value.domain),
            Domain::declared(
                &program.locals[read.local],
                &mut checker.flow,
                Span::default()
            )
            .unwrap()
            .as_ref()
        );
        assert_eq!(
            value.version,
            Some(Version {
                owner: read.owner,
                local: read.local,
                definition: Definition::Parameter(0)
            })
        );
        assert_eq!(value.origin, Some(Origin::Parameter(read.owner - 1, 0)));
        assert_eq!(
            snapshots.before(Port::Normal(point)),
            snapshots.after(Port::Normal(point))
        );
    }
    let (checker, _, _, snapshots) =
        capture("g:(){};g();x:=1;x;x=2;x;stable:3;stable;f<never>:(n<never>){->n}");
    for (&point, read) in &checker.local_reads {
        if read.normal {
            assert!(snapshots.expressions[&point].value.is_none());
        } else {
            assert!(!snapshots.expressions.contains_key(&point));
        }
    }
}

pub(super) fn definitions(
    state: &State,
    local: hir::LocalId,
    port: Port,
) -> definitions::Definitions {
    let Data::Initialized {
        version: Some(version),
        ..
    } = &state.get(local).unwrap().data
    else {
        panic!()
    };
    definitions::Definitions {
        declarations: [(
            local,
            definitions::Location {
                version: *version,
                port,
                normal: None,
            },
        )]
        .into_iter()
        .collect(),
        writes: BTreeMap::new(),
        ..definitions::Definitions::default()
    }
}

#[test]
pub(crate) fn scalar_expression_reads_preserve_copies_controls_and_missing_cell_states() {
    let (mut checker, program, reports, snapshots) = capture("f:(n<uint8>){n}");
    let local = program.functions[0].params[0];
    let entry = Port::BlockEntry(reports.entries[&1].0);
    let seed = snapshots.seed(entry).unwrap();
    let definitions = definitions(seed, local, entry);
    let observations = observations::Observations {
        declared: [(
            local,
            Domain::declared(&program.locals[local], &mut checker.flow, Span::default())
                .unwrap()
                .unwrap(),
        )]
        .into_iter()
        .collect(),
        ..observations::Observations::new(&definitions)
    };
    let mut state = seed
        .snapshot(&mut checker.flow, Span::default(), MAX_EDGES)
        .unwrap();
    let cell = state.cells.get_mut(&local).unwrap();
    let Data::Initialized { domain, .. } = &mut cell.data else {
        panic!()
    };
    *domain = Some(Domain::integer(7, &mut checker.flow, Span::default()).unwrap());
    cell.control = true;
    let first = snapshots
        .read(
            &state,
            local,
            false,
            &observations,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap()
        .unwrap();
    let Data::Initialized {
        domain, version, ..
    } = &mut state.cells.get_mut(&local).unwrap().data
    else {
        panic!()
    };
    *domain = Some(Domain::integer(9, &mut checker.flow, Span::default()).unwrap());
    *version = None;
    let later = snapshots
        .read(
            &state,
            local,
            true,
            &observations,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap()
        .unwrap();
    assert!(first.control && later.control);
    assert_ne!(first.domain, later.domain);
    assert_eq!(first.origin, later.origin);
    assert!(first.version.is_some());
    assert_eq!(later.version, None);
    assert_eq!(
        first.domain,
        Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
    );
    for data in [
        Data::Unavailable,
        Data::Uninitialized,
        Data::Partial,
        Data::Initialized {
            version: None,
            domain: None,
            origin: None,
        },
    ] {
        state.cells.get_mut(&local).unwrap().data = data;
        assert!(
            snapshots
                .read(
                    &state,
                    local,
                    false,
                    &observations,
                    &mut checker.flow,
                    Span::default()
                )
                .unwrap()
                .is_none()
        );
    }
    state.cells.remove(&local);
    assert!(
        snapshots
            .read(
                &state,
                local,
                false,
                &observations,
                &mut checker.flow,
                Span::default()
            )
            .unwrap()
            .is_none()
    );
}

#[test]
pub(crate) fn scalar_expression_reads_reject_foreign_future_and_out_of_type_values() {
    let (mut checker, program, reports, snapshots) = capture("f:(n<uint8>){n}");
    let local = program.functions[0].params[0];
    let entry = Port::BlockEntry(reports.entries[&1].0);
    let seed = snapshots.seed(entry).unwrap();
    let definitions = definitions(seed, local, entry);
    let observations = observations::Observations {
        declared: [(
            local,
            Domain::declared(&program.locals[local], &mut checker.flow, Span::default())
                .unwrap()
                .unwrap(),
        )]
        .into_iter()
        .collect(),
        ..observations::Observations::new(&definitions)
    };
    for fault in 0..7 {
        let mut state = seed
            .snapshot(&mut checker.flow, Span::default(), MAX_EDGES)
            .unwrap();
        let Data::Initialized {
            version,
            domain,
            origin,
        } = &mut state.cells.get_mut(&local).unwrap().data
        else {
            panic!()
        };
        match fault {
            0 => version.as_mut().unwrap().owner += 1,
            1 => version.as_mut().unwrap().local += 1,
            2 => version.as_mut().unwrap().definition = Definition::Parameter(1),
            3 => version.as_mut().unwrap().definition = Definition::Write(usize::MAX),
            4 => *domain = Some(Domain::boolean(true)),
            5 => *domain = Some(Domain::integer(256, &mut checker.flow, Span::default()).unwrap()),
            6 => *origin = Some(Origin::Parameter(9, 0)),
            _ => unreachable!(),
        }
        let before = format!("{snapshots:?}{state:?}");
        assert!(
            snapshots
                .read(
                    &state,
                    local,
                    false,
                    &observations,
                    &mut checker.flow,
                    Span::default()
                )
                .is_err(),
            "fault {fault}"
        );
        assert_eq!(before, format!("{snapshots:?}{state:?}"));
    }
}
