use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;

#[test]
pub(crate) fn declared_access_keeps_full_parameter_domains_without_calls() {
    let source = "f:(plain<{n<int32>}>,hidden<{n<&int32>}>,shared<&int32>,union<int32><null>){};g:(write<&!int32>){}";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let (access, parts) = checker
        .declared_access(&program, &reports, Span::default(), 5)
        .unwrap();
    assert_eq!(parts, 0);
    let params = &program.functions[0].params;
    assert_eq!(access[&params[0]], Access::Value);
    assert_eq!(access[&params[1]], Access::Unknown);
    assert!(matches!(access[&params[2]], Access::Shared(_)));
    assert!(matches!(
        access[&program.functions[1].params[0]],
        Access::Exclusive(_)
    ));
    assert_eq!(access[&params[3]], Access::Value);
    assert!(
        checker
            .declared_access(&program, &reports, Span::default(), 4)
            .is_err()
    );
}

mod views;

pub(super) fn context(source: &str) -> (Checker, Program, Reports, crate::borrow::Facts) {
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    checker.proofs.conditions = std::mem::take(&mut checker.guards);
    checker.proofs.tags = std::mem::take(&mut checker.tags);
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    (checker, program, reports, facts)
}
