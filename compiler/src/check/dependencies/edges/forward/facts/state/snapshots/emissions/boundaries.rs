use super::super::capture::tests::capture;
use super::*;

pub(super) fn setup() -> (
    Checker,
    Snapshots,
    hir::EmitId,
    observations::Emission,
    Value,
) {
    let (checker, _, _, mut snapshots) = capture("x:=7;{->x;x=9}");
    let (&statement, op) = checker.emissions.first_key_value().unwrap();
    let id = op.targets[0].id;
    let input = observations::Emission {
        owner: op.owner,
        statement,
        input: op.input,
        target: op.targets[0].block,
        reached: true,
        normal: true,
        control: Some(false),
    };
    let before = snapshots.rows[&Port::Emission(id)].before;
    snapshots.emissions.remove(&id).unwrap();
    (checker, snapshots, id, input, before)
}

#[test]
pub(crate) fn scalar_emission_boundaries_combine_source_and_entry_control_conservatively() {
    for (entry, dependent, expected) in [
        (Some(false), false, Some(false)),
        (Some(false), true, Some(true)),
        (Some(true), false, Some(true)),
        (None, false, None),
        (None, true, Some(true)),
    ] {
        let (mut checker, mut snapshots, id, mut input, before) = setup();
        input.control = entry;
        input.normal = false;
        snapshots
            .expressions
            .get_mut(&input.input)
            .unwrap()
            .value
            .as_mut()
            .unwrap()
            .control = dependent;
        let states = format!("{:?}{:?}", snapshots.rows, snapshots.states);
        let after = snapshots
            .emit(id, before, &input, None, &mut checker.flow, Span::default())
            .unwrap();
        assert_eq!(
            snapshots.emissions[&id]
                .value
                .as_ref()
                .map(|value| value.control),
            expected
        );
        assert_eq!(after, expected.map_or(Value::Missing, |_| before));
        assert_eq!(
            format!("{:?}{:?}", snapshots.rows, snapshots.states),
            states
        );
    }
}

#[test]
pub(crate) fn scalar_emission_boundaries_do_not_recover_absent_values_or_storage() {
    for fault in 0..4 {
        let (mut checker, mut snapshots, id, mut input, mut before) = setup();
        match fault {
            0 => {
                snapshots.expressions.remove(&input.input);
            }
            1 => snapshots.expressions.get_mut(&input.input).unwrap().value = None,
            2 => {
                snapshots.expressions.remove(&input.input);
                snapshots.rows.remove(&Port::Normal(input.input));
                input.reached = false;
                before = Value::Missing;
            }
            3 => {
                let row = snapshots.rows.get_mut(&Port::Normal(input.input)).unwrap();
                row.before = Value::Missing;
                row.after = Value::Missing;
                before = Value::Missing;
            }
            _ => unreachable!(),
        }
        assert_eq!(
            snapshots
                .emit(id, before, &input, None, &mut checker.flow, Span::default())
                .unwrap(),
            Value::Missing
        );
        assert!(snapshots.emissions[&id].value.is_none());
    }
}

#[test]
pub(crate) fn scalar_emission_boundaries_keep_known_bottom_distinct_from_missing() {
    let (checker, _, _, snapshots) = capture("f<int32>:(n<never>){->7}");
    let op = checker.emissions.values().next().unwrap();
    let id = op.targets[0].id;
    let value = snapshots.emissions[&id].value.as_ref().unwrap();
    assert_eq!(value.domain, Domain::BOTTOM);
    assert_eq!(value.local, None);
    assert_eq!(value.version, None);
    assert_eq!(value.origin, None);
    let row = &snapshots.rows[&Port::Emission(id)];
    assert_eq!(row.before, row.after);
    let Value::Known(state) = row.after else {
        panic!()
    };
    assert_eq!(snapshots.states[state].reach, FALSE);
}

#[test]
pub(crate) fn scalar_emission_boundaries_reject_stale_histories_without_publication() {
    for fault in 0..6 {
        let (mut checker, mut snapshots, id, mut input, before) = setup();
        match fault {
            0 => input.owner += 1,
            1 => input.reached = false,
            2 => input.input = input.statement,
            3 => snapshots.expressions.get_mut(&input.input).unwrap().owner += 1,
            4 => {
                snapshots
                    .rows
                    .get_mut(&Port::Normal(input.input))
                    .unwrap()
                    .after = Value::Missing
            }
            5 => {
                snapshots
                    .expressions
                    .get_mut(&input.input)
                    .unwrap()
                    .value
                    .as_mut()
                    .unwrap()
                    .domain
                    .values = Some(domain::Values::Bool(0))
            }
            _ => unreachable!(),
        }
        let saved = format!("{snapshots:?}");
        assert!(
            snapshots
                .emit(id, before, &input, None, &mut checker.flow, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{snapshots:?}"), saved);
    }
}

#[test]
pub(crate) fn scalar_emission_boundaries_reserve_exact_budget_and_reject_duplicate_publication() {
    let (mut checker, mut snapshots, id, input, before) = setup();
    let start = checker.flow.work;
    snapshots
        .emit(id, before, &input, None, &mut checker.flow, Span::default())
        .unwrap();
    let cost = checker.flow.work - start;
    let fact = snapshots.emissions.remove(&id).unwrap();
    let weight = fact.weight();
    for (room, extra, pass) in [
        (weight - 1, 0, false),
        (weight, 1, false),
        (weight, 0, true),
    ] {
        snapshots.parts = room;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = snapshots.emit(id, before, &input, None, &mut checker.flow, Span::default());
        assert_eq!(result.is_ok(), pass);
        assert_eq!(snapshots.emissions.get(&id), pass.then_some(&fact));
        assert_eq!(snapshots.parts, if pass { 0 } else { room });
    }
    checker.flow.work = 0;
    snapshots.parts = MAX_EDGES;
    let saved = format!("{snapshots:?}");
    assert!(
        snapshots
            .emit(id, before, &input, None, &mut checker.flow, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{snapshots:?}"), saved);
}
