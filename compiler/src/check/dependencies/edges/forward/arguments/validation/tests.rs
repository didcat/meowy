use super::{super::tests::context, *};

#[test]
pub(crate) fn argument_row_validation_accepts_conservative_missing_observations() {
    for state in 0..4 {
        let (mut checker, mut reports) = context("f:(a<int32>,b<int32>){};f(1,2)");
        let point = *reports.arguments.first_key_value().unwrap().0;
        match state {
            0 => reports.arguments.get_mut(&point).unwrap().entry = false,
            1 => reports.arguments.get_mut(&point).unwrap().normal = false,
            2 => {
                reports.arguments.remove(&point);
            }
            3 => reports.arguments.clear(),
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        checker
            .validate_call_arguments(&reports, Span::default())
            .unwrap();
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_row_validation_rejects_stale_identities_and_invented_flags() {
    for fault in 0..12 {
        let (mut checker, mut reports) = context("f:(a<int32>,b<int32>){};f(1,2)");
        let (&point, &arg) = reports.arguments.first_key_value().unwrap();
        match fault {
            0 => reports.arguments.get_mut(&point).unwrap().site = usize::MAX,
            1 => reports.arguments.get_mut(&point).unwrap().position += 1,
            2 => reports.arguments.get_mut(&point).unwrap().param = reports.locals,
            3 => reports.arguments.get_mut(&point).unwrap().owner += 1,
            4 => {
                let row = reports.arguments.get_mut(&point).unwrap();
                row.entry = false;
                row.normal = false;
            }
            5 => {
                reports.arguments.remove(&point);
                let other = *reports.arguments.first_key_value().unwrap().0;
                reports.arguments.insert(other, arg);
            }
            6 => {
                reports.arguments.insert(usize::MAX, arg);
            }
            7 => reports
                .entries
                .get_mut(&arg.owner)
                .unwrap()
                .1
                .ports
                .retain(|port| *port != Port::Normal(point)),
            8 => reports
                .entries
                .get_mut(&arg.owner)
                .unwrap()
                .1
                .ports
                .retain(|port| *port != Port::Entry(point)),
            9 => {
                checker.invocations.remove(&arg.site);
            }
            10 => reports.signatures.params.get_mut(&arg.param).unwrap().1 += 1,
            11 => checker.points[point].parent = None,
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}{:?}",
            checker.points,
            checker.invocations,
            checker.edge_counts()
        );
        let error = checker
            .validate_call_arguments(&reports, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity mismatch"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}{:?}",
                checker.points,
                checker.invocations,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn argument_row_validation_charges_exact_work_without_reserving_payload() {
    let (mut checker, reports) = context("f:(a<int32>,b<int32>){};f(1,2)");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    checker
        .validate_call_arguments(&reports, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.validate_call_arguments(&reports, Span::default());
        if short == 0 {
            result.unwrap();
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
            assert!(checker.flow.exceeded());
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
