use super::{tests::checked, *};
use crate::check::dependencies::Invocation;
use std::collections::BTreeSet;

#[test]
pub(crate) fn function_context_keeps_parameter_order_and_identity_after_hir_transfer() {
    let source = "root:1;f<uint8>:(a<uint8>,unused<boolean>,b<int32>){->a};alias:f;f(7,true,9);empty:(){};unused<int32>:(ignored<int32>){->3};recur<int32>:(n<int32>){->recur(n)}";
    crate::compile(source).unwrap();
    let (mut checker, mut program) = checked(source);
    assert!(checker.functions.is_empty() && checker.locals.is_empty());
    let f = program
        .functions
        .iter()
        .find(|function| function.name == "f")
        .unwrap();
    let expected = f.params.clone();
    assert_eq!(expected.len(), 3);
    assert_eq!(
        program.locals[expected[0]],
        crate::hir::Type::Int {
            bits: 8,
            signed: false
        }
    );
    assert_eq!(program.locals[expected[1]], crate::hir::Type::Bool);
    assert_eq!(
        program.locals[expected[2]],
        crate::hir::Type::Int {
            bits: 32,
            signed: true
        }
    );
    assert!(
        !checker
            .local_reads
            .values()
            .any(|read| read.local == expected[1])
    );
    program.functions.reverse();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let params = checker
        .initializer_parameters(&program, &reports, Span::default())
        .unwrap();
    let mut seen = BTreeSet::new();
    for function in &program.functions {
        assert_eq!(reports.entries[&(function.id + 1)].0, function.body.id);
        for local in &function.params {
            assert!(seen.insert(*local));
            assert!(params.contains(local));
            assert!(!reports.initializers.contains_key(local));
            assert!(checker.proofs.bindings.contains_key(local));
        }
    }
    for call in checker.invocations.values() {
        let function = program
            .functions
            .iter()
            .find(|function| function.id == call.function)
            .unwrap();
        assert_eq!(call.args.len(), function.params.len());
        assert_eq!(call.shape, Invocation::result_shape(&function.result));
    }
    assert_eq!(params, seen);
    assert_eq!(reports.entries.len(), program.functions.len() + 1);
    assert_eq!(
        program
            .functions
            .iter()
            .find(|function| function.name == "f")
            .unwrap()
            .params,
        expected
    );
}

#[test]
pub(crate) fn function_context_keeps_forward_groups_aliases_and_recursive_owners() {
    let source = "even<(uint32)->boolean>;odd<(uint32)->boolean>;even<boolean>:(n<uint32>)'out{|n==0|{'out->true;'out.leave()};->odd(n-1)};odd<boolean>:(n<uint32>)'out{|n==0|{'out->false;'out.leave()};->even(n-1)};alias:even;x:4.(alias)";
    crate::compile(source).unwrap();
    let (mut checker, mut program) = checked(source);
    program.functions.reverse();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    assert_eq!(reports.calls.sites.len(), 3);
    let mut owners = BTreeSet::new();
    for call in checker.invocations.values() {
        let function = program
            .functions
            .iter()
            .find(|function| function.id == call.function)
            .unwrap();
        assert_eq!(call.args.len(), function.params.len());
        assert_eq!(call.shape, Invocation::result_shape(&function.result));
        let param = function.params[0];
        assert!(
            checker
                .local_reads
                .values()
                .any(|read| read.local == param && read.owner == function.id + 1)
        );
        assert_eq!(reports.calls.sites[&call.site].callee, function.id + 1);
        owners.insert(call.owner);
    }
    assert_eq!(owners, BTreeSet::from([0, 1, 2]));
}

#[test]
pub(crate) fn function_context_preserves_signature_alias_compatibility_boundaries() {
    let source = "f<uint8>:(a<uint8>,unused<boolean>){->a};->alias<(uint8,boolean)->uint8>:f;x:alias(7,true)";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    assert_eq!(program.functions.len(), 1);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let call = checker.invocations.first_key_value().unwrap().1;
    assert_eq!(call.function, program.functions[0].id);
    assert_eq!(call.args.len(), program.functions[0].params.len());
    assert_eq!(
        call.shape,
        Invocation::result_shape(&program.functions[0].result)
    );
    assert_eq!(
        reports.calls.sites[&call.site].callee,
        program.functions[0].id + 1
    );
    let error = crate::compile("f<int32>:(n<int32>){->n};alias<(int32)->int32>:f;alias(1)")
        .unwrap_err()
        .remove(0);
    assert_eq!(error.code, "B001");
    assert_eq!(
        error.message,
        "annotated or mutable compile-time identity bindings is not supported by this bootstrap compiler"
    );
    for source in [
        "f<(int32)->int32>;f<int32>:(n<uint8>){->1}",
        "f<(int32)->int32>;f<boolean>:(n<int32>){->true}",
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, "E221");
    }
}
