use super::super::capture::tests::capture;
use super::*;

pub(super) fn context() -> (
    Checker,
    Reports,
    Snapshots,
    definitions::Definitions,
    PointId,
    hir::LocalId,
) {
    let (checker, program, reports, snapshots) = capture("7;f:(n<uint8>){n}");
    let point = *checker.local_reads.first_key_value().unwrap().0;
    let local = program.functions[0].params[0];
    let version = snapshots.expressions[&point]
        .value
        .as_ref()
        .unwrap()
        .version
        .unwrap();
    let definitions = definitions::Definitions {
        declarations: [(
            local,
            definitions::Location {
                version,
                port: Port::BlockEntry(reports.entries[&1].0),
                normal: None,
            },
        )]
        .into_iter()
        .collect(),
        writes: BTreeMap::new(),
    };
    (checker, reports, snapshots, definitions, point, local)
}

#[test]
pub(crate) fn scalar_read_results_bound_publication_without_changing_existing_histories() {
    let (mut checker, reports, mut snapshots, definitions, point, local) = context();
    let expected = snapshots.expressions.remove(&point).unwrap();
    let observations = observations::Observations {
        sources: [(Port::Normal(point), &reports.effects[&point])]
            .into_iter()
            .collect(),
        declared: [(local, expected.value.as_ref().unwrap().domain.clone())]
            .into_iter()
            .collect(),
        ..observations::Observations::new(&definitions)
    };
    snapshots.parts = MAX_EDGES;
    let start = checker.flow.work;
    snapshots
        .expression(
            Port::Normal(point),
            &observations,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - snapshots.room();
    assert_eq!(used, expected.weight());
    assert_eq!(snapshots.expressions.remove(&point), Some(expected));
    let prior = format!(
        "{:?}{:?}{:?}",
        snapshots.states, snapshots.rows, snapshots.expressions
    );
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        snapshots.parts = room;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = snapshots.expression(
            Port::Normal(point),
            &observations,
            &mut checker.flow,
            Span::default(),
        );
        assert_eq!(result.is_ok(), pass);
        assert_eq!(snapshots.room(), if pass { 0 } else { room });
        assert_eq!(snapshots.expressions.remove(&point).is_some(), pass);
        assert_eq!(
            format!(
                "{:?}{:?}{:?}",
                snapshots.states, snapshots.rows, snapshots.expressions
            ),
            prior
        );
    }
}

#[test]
pub(crate) fn scalar_read_results_reject_missing_stale_and_duplicate_records_atomically() {
    for fault in 0..8 {
        let (mut checker, mut reports, mut snapshots, definitions, point, local) = context();
        let declared = snapshots.expressions[&point]
            .value
            .as_ref()
            .unwrap()
            .domain
            .clone();
        if fault != 7 {
            snapshots.expressions.remove(&point);
        }
        let port = Port::Normal(point);
        match fault {
            0 => {
                snapshots.rows.remove(&port);
            }
            1 => snapshots.rows.get_mut(&port).unwrap().owner += 1,
            2 => snapshots.rows.get_mut(&port).unwrap().after = Value::Missing,
            3 => {
                let (_, Effect::Read { normal, .. }) = reports.effects.get_mut(&point).unwrap()
                else {
                    panic!()
                };
                *normal = false;
            }
            4 => {
                snapshots
                    .rows
                    .remove(&definitions.declarations[&local].port);
            }
            5 => {
                let row = snapshots.rows.get_mut(&port).unwrap();
                row.before = Value::Known(usize::MAX);
                row.after = row.before;
            }
            _ => {}
        }
        let observations = observations::Observations {
            sources: [(port, &reports.effects[&point])].into_iter().collect(),
            declared: if fault == 6 {
                HashMap::new()
            } else {
                [(local, declared)].into_iter().collect()
            },
            ..observations::Observations::new(&definitions)
        };
        let before = format!("{snapshots:?}");
        assert!(
            snapshots
                .expression(port, &observations, &mut checker.flow, Span::default())
                .is_err(),
            "fault {fault}"
        );
        assert_eq!(format!("{snapshots:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_read_results_keep_missing_observations_inputs_and_bottom_distinct() {
    let (mut checker, reports, mut snapshots, definitions, point, local) = context();
    let expected = snapshots.expressions.remove(&point).unwrap();
    let mut observations = observations::Observations {
        sources: [(Port::Operation(point), &reports.effects[&point])]
            .into_iter()
            .collect(),
        declared: [(local, expected.value.as_ref().unwrap().domain.clone())]
            .into_iter()
            .collect(),
        ..observations::Observations::new(&definitions)
    };
    for port in [Port::Operation(point), Port::Normal(point)] {
        snapshots
            .expression(port, &observations, &mut checker.flow, Span::default())
            .unwrap();
        assert!(!snapshots.expressions.contains_key(&point));
    }
    let port = Port::Normal(point);
    observations.sources.insert(port, &reports.effects[&point]);
    let row = snapshots.rows.get_mut(&port).unwrap();
    let before = row.before;
    row.before = Value::Missing;
    row.after = Value::Missing;
    snapshots
        .expression(port, &observations, &mut checker.flow, Span::default())
        .unwrap();
    assert!(
        snapshots
            .expressions
            .remove(&point)
            .unwrap()
            .value
            .is_none()
    );
    let row = snapshots.rows.get_mut(&port).unwrap();
    row.before = before;
    row.after = before;
    let Value::Known(id) = before else { panic!() };
    snapshots.states[id].reach = FALSE;
    snapshots.states[id].cells.clear();
    snapshots.states[id].parts = 1;
    snapshots
        .expression(port, &observations, &mut checker.flow, Span::default())
        .unwrap();
    let value = snapshots.expressions[&point].value.as_ref().unwrap();
    assert_eq!(value.domain, Domain::BOTTOM);
    assert_eq!(value.version, None);
    assert_eq!(value.origin, None);
}
