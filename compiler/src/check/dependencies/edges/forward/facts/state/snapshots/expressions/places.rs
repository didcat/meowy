use super::super::capture::tests::capture;
use super::*;
use domain::Values;

#[test]
pub(crate) fn scalar_read_places_keep_equal_merged_values_in_distinct_locals() {
    let (checker, _, _, snapshots) =
        capture("f:(a<boolean>,b<boolean>){x:=a;y:=b;|a|x=true;|b|y=false;x;y}");
    let reads = checker.local_reads.iter().rev().take(2).collect::<Vec<_>>();
    let mut locals = Vec::new();
    for (point, read) in reads {
        let value = snapshots.expressions[point].value.as_ref().unwrap();
        assert_eq!(value.local, Some(read.local));
        assert_eq!(value.version, None);
        assert_eq!(value.origin, None);
        assert_eq!(value.domain.values, Some(Values::Bool(3)));
        locals.push(value.local);
    }
    assert_eq!(locals.len(), 2);
    assert_ne!(locals[0], locals[1]);
}

#[test]
pub(crate) fn scalar_read_places_do_not_copy_storage_identity_from_value_origins() {
    let (checker, _, _, snapshots) = capture("f:(b<boolean>){copy:b;b;copy}");
    let values = checker
        .local_reads
        .iter()
        .rev()
        .take(2)
        .map(|(point, read)| {
            let value = snapshots.expressions[point].value.as_ref().unwrap();
            assert_eq!(value.local, Some(read.local));
            value
        })
        .collect::<Vec<_>>();
    assert_eq!(values[0].origin, values[1].origin);
    assert_ne!(values[0].local, values[1].local);
    assert_ne!(values[0].version, values[1].version);
}

#[test]
pub(crate) fn scalar_read_places_survive_transparent_wrappers_and_keep_missing_sources_absent() {
    let (checker, _, _, snapshots) =
        capture("f:(b<boolean>,c<boolean>){x:=b;|c|x=true;((x)~<boolean>);x}");
    let read = *checker.local_reads.keys().next_back().unwrap();
    let value = snapshots.expressions[&read].value.as_ref().unwrap();
    assert_eq!(value.version, None);
    for point in checker.group_inputs.keys().chain(checker.typed_ops.keys()) {
        assert_eq!(
            snapshots.expressions[point].value.as_ref().unwrap().local,
            value.local
        );
    }
    let (checker, _, _, snapshots) = capture("row:{->x:2;x};f:(p<&int32>,text<string>){p;text}");
    for point in checker.local_reads.keys() {
        assert!(
            snapshots
                .expressions
                .get(point)
                .is_none_or(|fact| fact.value.is_none())
        );
    }
}

#[test]
pub(crate) fn scalar_read_places_charge_forwarded_identity_without_changing_histories() {
    let (mut checker, _, _, mut snapshots) = capture("f:(b<boolean>){(b)}");
    let (&point, group) = checker.group_inputs.first_key_value().unwrap();
    let forward = observations::Forward {
        owner: group.owner,
        input: group.input,
        control: false,
    };
    let expected = snapshots.expressions.remove(&point).unwrap();
    assert!(expected.value.as_ref().unwrap().local.is_some());
    let start = checker.flow.work;
    snapshots
        .forward_expression(point, &forward, &mut checker.flow, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(
        snapshots.expressions.remove(&point),
        Some(Fact {
            owner: expected.owner,
            value: expected.value.clone()
        })
    );
    let before = format!(
        "{:?}{:?}{:?}",
        snapshots.states, snapshots.rows, snapshots.expressions
    );
    for (room, extra, pass) in [
        (expected.weight() - 1, 0, false),
        (expected.weight(), 1, false),
        (expected.weight(), 0, true),
    ] {
        snapshots.parts = room;
        checker.flow.full = false;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + extra;
        assert_eq!(
            snapshots
                .forward_expression(point, &forward, &mut checker.flow, Span::default())
                .is_ok(),
            pass
        );
        assert_eq!(snapshots.room(), if pass { 0 } else { room });
        let result = snapshots.expressions.remove(&point);
        if pass {
            assert_eq!(result.as_ref(), Some(&expected));
        } else {
            assert!(result.is_none());
        }
        assert_eq!(
            format!(
                "{:?}{:?}{:?}",
                snapshots.states, snapshots.rows, snapshots.expressions
            ),
            before
        );
    }
}
