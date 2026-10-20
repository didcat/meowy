use super::super::capture::tests::capture;
use super::tests::setup;
use super::*;
use crate::check::dependencies::OperationKind;

#[test]
pub(crate) fn scalar_storage_writes_distinguish_unknown_values_incomplete_cells_and_missing_storage()
 {
    for data in [Data::Unavailable, Data::Partial, Data::Uninitialized] {
        let (mut checker, mut snapshots, definitions, _) = setup("x:=1;x=2");
        let point = *definitions.writes.keys().next().unwrap();
        let storage = &definitions.storage[&point];
        let Value::Known(id) = snapshots.rows[&Port::Operation(point)].before else {
            panic!()
        };
        let mut state = snapshots.states[id]
            .snapshot(&mut checker.flow, Span::default(), snapshots.room())
            .unwrap();
        state
            .put(
                storage.version.local,
                &Cell {
                    data,
                    control: true,
                },
                &mut checker.flow,
                Span::default(),
                snapshots.room(),
            )
            .unwrap();
        let before = snapshots.keep(state, Span::default()).unwrap();
        snapshots
            .rows
            .get_mut(&Port::Normal(storage.input))
            .unwrap()
            .after = before;
        snapshots.expressions.remove(&storage.input);
        let Some(Value::Known(id)) = snapshots
            .storage(
                point,
                before,
                &definitions,
                &mut checker.flow,
                Span::default(),
            )
            .unwrap()
        else {
            panic!()
        };
        let cell = snapshots.states[id].get(storage.version.local).unwrap();
        assert!(cell.control);
        assert_eq!(
            cell.data,
            Data::Initialized {
                version: Some(storage.version),
                domain: Some(storage.declared.clone()),
                origin: Some(Origin::Version(storage.version)),
            }
        );
        let empty = State::new(
            storage.version.owner,
            crate::flow::TRUE,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
        let missing = snapshots.keep(empty, Span::default()).unwrap();
        assert_eq!(
            snapshots
                .storage(
                    point,
                    missing,
                    &definitions,
                    &mut checker.flow,
                    Span::default()
                )
                .unwrap(),
            Some(Value::Missing)
        );
    }
}

#[test]
pub(crate) fn scalar_storage_transfers_preserve_rhs_and_operation_controls() {
    for write in [false, true] {
        for source in [false, true] {
            let (mut checker, mut snapshots, mut definitions, point) = setup("x:=1;x=2");
            let point = if write {
                *definitions.writes.keys().next().unwrap()
            } else {
                point
            };
            let storage = definitions.storage.get_mut(&point).unwrap();
            let before = snapshots.rows[&Port::Operation(point)].before;
            if source {
                snapshots
                    .expressions
                    .get_mut(&storage.input)
                    .unwrap()
                    .value
                    .as_mut()
                    .unwrap()
                    .control = true;
            } else {
                storage.control = true;
            }
            let local = storage.version.local;
            let Some(Value::Known(id)) = snapshots
                .storage(
                    point,
                    before,
                    &definitions,
                    &mut checker.flow,
                    Span::default(),
                )
                .unwrap()
            else {
                panic!()
            };
            assert!(snapshots.states[id].get(local).unwrap().control);
        }
    }
}

#[test]
pub(crate) fn scalar_storage_capture_keeps_stopped_rhs_and_bottom_frames_distinct() {
    let (checker, _, reports, snapshots) = capture(
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};f<never>:(n<int32>){x:=n;copy:x;x=stop();late:x}",
    );
    let (&point, write) = checker
        .operations
        .iter()
        .find(|(_, op)| op.kind == OperationKind::Write)
        .unwrap();
    assert!(!reports.effects.contains_key(&point));
    assert_eq!(snapshots.before(Port::Operation(point)), None);
    assert_eq!(snapshots.after(Port::Operation(point)), None);
    let reads = checker
        .local_reads
        .iter()
        .filter(|(_, read)| read.local == write.storage)
        .collect::<Vec<_>>();
    assert_eq!(reads.len(), 2);
    assert!(snapshots.expressions[reads[0].0].value.is_some());
    assert!(!snapshots.expressions.contains_key(reads[1].0));
    let (_, _, _, snapshots) = capture("f:(n<never>){x:=1;x=2}");
    let mut bottom = 0;
    for row in snapshots.rows.values().filter(|row| row.owner == 1) {
        assert_eq!(row.before, row.after);
        if let Value::Known(id) = row.after {
            assert_eq!(snapshots.states[id].reach, FALSE);
            bottom += 1;
        }
    }
    assert!(bottom > 1);
}
