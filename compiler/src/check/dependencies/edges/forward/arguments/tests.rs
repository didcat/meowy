use super::{super::entries::tests::checked, *};

mod forests;

pub(super) fn context(source: &str) -> (Checker, Reports) {
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    (checker, reports)
}

#[test]
pub(crate) fn argument_observations_leave_empty_calls_and_unused_parameters_without_rows() {
    for source in ["", "empty:(){};empty()", "unused:(p<int32>){}"] {
        let (mut checker, reports) = context(source);
        assert!(reports.arguments.is_empty());
        assert_eq!(
            checker
                .call_arguments(&reports, Span::default(), 0, 17)
                .unwrap(),
            (Arguments::new(), 17)
        );
    }
}

#[test]
pub(crate) fn argument_observations_retain_original_sites_positions_and_owners() {
    let (checker, reports) = context("f:(a<int32>,unused<boolean>){};g:(){f(1,true)};f(2,false)");
    assert_eq!(reports.arguments.len(), 4);
    assert!(reports.arguments.values().any(|arg| arg.owner != 0));
    for call in checker.invocations.values() {
        let params = &reports.signatures.functions[&call.function].params;
        for (position, (param, point)) in params.iter().zip(&call.args).enumerate() {
            assert_eq!(
                reports.arguments[point],
                Argument {
                    site: call.site,
                    position,
                    param: *param,
                    owner: call.owner,
                    entry: true,
                    normal: true
                }
            );
            assert!(!reports.initializers.contains_key(param));
        }
    }
}

#[test]
pub(crate) fn argument_observations_keep_stopped_prefixes_without_call_effects() {
    for (args, stopped) in [("d.panic(\"stop\"),2,3", 0), ("1,d.panic(\"stop\"),3", 1)] {
        let source = format!("d:@\"debug\";f:(a<int32>,b<int32>,c<int32>){{}};f({args})");
        let (checker, reports) = context(&source);
        let call = checker.invocations.first_key_value().unwrap().1;
        assert_eq!(reports.arguments.len(), stopped + 1);
        assert!(!reports.effects.contains_key(&call.point));
        assert!(!reports.calls.sites.contains_key(&call.site));
        for (position, point) in call.args.iter().enumerate() {
            if position <= stopped {
                let arg = reports.arguments[point];
                assert!(arg.entry);
                assert_eq!(arg.normal, position < stopped);
                assert_eq!(arg.position, position);
            } else {
                assert!(!reports.arguments.contains_key(point));
            }
        }
    }
}

#[test]
pub(crate) fn argument_observations_merge_partial_ports_and_ignore_call_visits() {
    let (mut checker, mut reports) = context("f:(a<int32>,b<int32>){};f(1,2)");
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    reports.effects.remove(&call.point);
    reports.entries.get_mut(&call.owner).unwrap().1.ports = vec![
        Port::Entry(call.args[0]),
        Port::Entry(call.args[0]),
        Port::Normal(call.args[1]),
    ];
    let mut expected = reports.arguments.clone();
    expected.get_mut(&call.args[0]).unwrap().normal = false;
    expected.get_mut(&call.args[1]).unwrap().entry = false;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert_eq!(
        checker
            .call_arguments(&reports, Span::default(), 2, 2)
            .unwrap(),
        (expected, 0)
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    reports.entries.get_mut(&call.owner).unwrap().1.ports =
        vec![Port::Operation(call.point), Port::Normal(call.point)];
    assert_eq!(
        checker
            .call_arguments(&reports, Span::default(), 0, 0)
            .unwrap(),
        (Arguments::new(), 0)
    );
}
