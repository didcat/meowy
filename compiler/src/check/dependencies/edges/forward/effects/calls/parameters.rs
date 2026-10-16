use super::{super::super::entries::tests::checked, *};

#[test]
pub(crate) fn call_parameter_context_keeps_original_order_after_hir_transfer() {
    let source = "f<uint8>:(a<uint8>,unused<boolean>,b<int32>){->a};->alias<(uint8,boolean,int32)->uint8>:f;x:7.(alias,true,9);empty:(){};empty();recur<int32>:(n<int32>){->recur(n)};unused:(p<boolean>){}";
    crate::compile(source).unwrap();
    let (mut checker, mut program) = checked(source);
    program.functions.reverse();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    assert!(checker.functions.is_empty() && checker.locals.is_empty());
    assert_eq!(reports.signatures.functions.len(), 4);
    let sites = checker.invocations.keys().copied().collect::<Vec<_>>();
    assert_eq!(sites.len(), 3);
    for site in sites {
        let start = checker.flow.work;
        let call = checker
            .checked_call_effect(&reports, site, Span::default())
            .unwrap()
            .clone();
        let function = program
            .functions
            .iter()
            .find(|function| function.id == call.function)
            .unwrap();
        let header = &reports.signatures.functions[&function.id];
        assert_eq!(header.params, function.params);
        assert_eq!(header.params.len(), call.args.len());
        for (position, (param, arg)) in header.params.iter().zip(&call.args).enumerate() {
            assert_eq!(reports.signatures.params[param], (function.id, position));
            assert!(!reports.initializers.contains_key(param));
            assert!(!reports.shared_initializers.contains_key(param));
            assert_eq!(checker.points[*arg].owner, call.owner);
            assert_eq!(checker.points[*arg].parent, Some(call.point));
        }
        if function.name == "f" {
            assert_eq!(call.owner, 0);
            let args = call
                .args
                .iter()
                .map(|arg| {
                    let span = checker.points[*arg].span;
                    &source[span.start..span.end]
                })
                .collect::<Vec<_>>();
            assert_eq!(args, ["7", "true", "9"]);
        }
        let count = call.args.len();
        println!(
            "{}: {count} arguments, {} work",
            function.name,
            checker.flow.work - start
        );
    }
}

#[test]
pub(crate) fn call_parameter_context_preserves_partial_reports_and_stopped_arguments() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};f<int32>:(a<int32>,unused<boolean>){->a};x:f(stop(),true)";
    let (mut checker, program) = checked(source);
    let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
    let outer = checker
        .invocations
        .values()
        .find(|call| call.args.len() == 2)
        .unwrap()
        .clone();
    assert!(!reports.effects.contains_key(&outer.point));
    assert!(
        !reports.entries[&outer.owner]
            .1
            .ports
            .contains(&Port::Entry(outer.args[1]))
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let call = checker
        .checked_call_effect(&reports, outer.site, Span::default())
        .unwrap();
    assert_eq!(call, &outer);
    assert_eq!(reports.signatures.functions[&call.function].params.len(), 2);
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    for (operation, result) in [(true, false), (false, true), (true, true)] {
        let effect = Effect::Call {
            site: outer.site,
            function: outer.function,
            args: outer.args.clone(),
            shape: outer.shape,
            may_return: outer.may_return,
            control: outer.control,
            operation,
            result,
        };
        checker
            .validate_call_report(&reports, outer.point, outer.owner, &effect, Span::default())
            .unwrap();
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    reports
        .signatures
        .functions
        .get_mut(&outer.function)
        .unwrap()
        .params
        .swap(0, 1);
    assert!(
        checker
            .checked_call_effect(&reports, outer.site, Span::default())
            .unwrap_err()
            .message
            .contains("function-signature identity")
    );
}
