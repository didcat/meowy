use super::{super::super::tests::checked, *};

pub(super) fn source() -> &'static str {
    "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:{->7;->tag:true};|v<boolean>|xs<T[2]><U[2]>:[r,v]}"
}

#[test]
pub(crate) fn list_reports_qualify_independent_observations_with_construction_registration() {
    let (mut checker, mut reports) = checked(source(), false);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let Effect::List(full) = reports.effects[&id].1.clone() else {
        panic!()
    };
    assert!(full.inputs[0].projected && full.inputs[0].converted);
    let read = checker
        .unchanged_narrowing_input(&reports, full.inputs[0].point, owner, Span::default())
        .unwrap()
        .unwrap();
    let root = checker
        .read_initializer_input(&reports, read, owner, Span::default())
        .unwrap()
        .unwrap();
    assert_eq!(reports.consumers[&root].0, owner);
    for state in 0..4 {
        let mut observed = full.clone();
        for input in &mut observed.inputs {
            input.projected = false;
            input.converted = false;
        }
        observed.inputs[0].projected = state == 0;
        observed.inputs[0].converted = state == 1;
        observed.constructed = state == 2;
        observed.result = state == 3;
        let before = format!("{reports:?}");
        checker
            .validate_list_report(&reports, id, owner, &observed, Span::default())
            .unwrap();
        assert_eq!(format!("{reports:?}"), before);
        reports.index.operations.remove(&id);
        assert!(
            checker
                .validate_list_report(&reports, id, owner, &observed, Span::default())
                .is_err()
        );
        reports.index.operations.insert(id, owner);
    }
}

#[test]
pub(crate) fn list_reports_reject_headers_and_unobserved_input_corruption_atomically() {
    for fault in 0..12 {
        let (mut checker, reports) = checked(source(), false);
        let (&id, op) = checker.lists.first_key_value().unwrap();
        let owner = op.owner;
        let Effect::List(mut observed) = reports.effects[&id].1.clone() else {
            panic!()
        };
        match fault {
            0 => observed.capacity += 1,
            1 => observed.contextual = false,
            2 => observed.normal = false,
            3 => observed.control = !observed.control,
            4 => {
                observed.inputs.pop();
            }
            5 => observed.inputs[1].point = id,
            6 => observed.inputs[1].plan = None,
            7 => observed.inputs[0].plan.as_mut().unwrap().0 = false,
            8 => observed.inputs[0].plan.as_mut().unwrap().1 = CoercionKind::Forward,
            9 => observed.inputs[1].projected = true,
            10 => {
                for input in &mut observed.inputs {
                    input.projected = false;
                    input.converted = false;
                }
                observed.constructed = false;
                observed.result = false;
            }
            11 => {
                observed.inputs[1].point = usize::MAX;
                observed.inputs[1].projected = false;
                observed.inputs[1].converted = false;
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.list_inputs);
        assert!(
            checker
                .validate_list_report(&reports, id, owner, &observed, Span::default())
                .is_err(),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.list_inputs), before);
    }
}
