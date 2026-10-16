use super::{super::super::super::entries::tests::checked, *};

#[test]
pub(crate) fn call_parameter_pairs_preserve_wrapped_roots_and_reference_parameters() {
    let source = "f<int32>:(a<int32>,p<&int32>,unused<boolean>){->a+*p};->alias<(int32,&int32,boolean)->int32>:f;n:5;r:{->7;->tag:true};x:((r)).(alias,&n,true)";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    let function = program
        .functions
        .iter()
        .find(|function| function.id == call.function)
        .unwrap();
    let expected = function
        .params
        .iter()
        .copied()
        .zip(call.args.iter().copied())
        .collect::<Vec<_>>();
    let before = format!(
        "{reports:?}{:?}{:?}",
        checker.invocations,
        checker.edge_counts()
    );
    let inputs = checker
        .checked_call_inputs(&reports, call.site, Span::default())
        .unwrap();
    assert_eq!(inputs.call, &call);
    assert_eq!(inputs.pairs.len(), 3);
    assert_eq!(inputs.pairs.collect::<Vec<_>>(), expected);
    let texts = expected
        .iter()
        .map(|(_, arg)| {
            let span = checker.points[*arg].span;
            &source[span.start..span.end]
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, ["((r))", "&n", "true"]);
    assert!(matches!(
        program.locals[function.params[1]],
        hir::Type::Reference(_)
    ));
    for (param, arg) in expected {
        assert_eq!(checker.points[arg].owner, call.owner);
        assert!(!reports.initializers.contains_key(&param));
        assert!(!reports.shared_initializers.contains_key(&param));
    }
    assert_eq!(
        format!(
            "{reports:?}{:?}{:?}",
            checker.invocations,
            checker.edge_counts()
        ),
        before
    );
}

#[test]
pub(crate) fn call_parameter_pairs_keep_forward_and_empty_function_identity() {
    let source = "f<(int32)->int32>;g<(int32)->int32>;g<int32>:(n<int32>){->n};f<int32>:(x<int32>){->g(x)};empty:(){};empty();x:f(2)";
    crate::compile(source).unwrap();
    let (mut checker, mut program) = checked(source);
    program.functions.reverse();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let calls = checker.invocations.values().cloned().collect::<Vec<_>>();
    assert_eq!(calls.len(), 3);
    let before = format!(
        "{reports:?}{:?}{:?}",
        checker.invocations,
        checker.edge_counts()
    );
    for call in calls {
        let function = program
            .functions
            .iter()
            .find(|function| function.id == call.function)
            .unwrap();
        let inputs = checker
            .checked_call_inputs(&reports, call.site, Span::default())
            .unwrap();
        assert_eq!(inputs.pairs.len(), function.params.len());
        assert_eq!(
            inputs.pairs.collect::<Vec<_>>(),
            function
                .params
                .iter()
                .copied()
                .zip(call.args.iter().copied())
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(
        format!(
            "{reports:?}{:?}{:?}",
            checker.invocations,
            checker.edge_counts()
        ),
        before
    );
}
