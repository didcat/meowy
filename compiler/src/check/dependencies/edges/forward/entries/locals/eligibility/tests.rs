use super::super::super::tests::checked;
use super::*;

#[test]
pub(crate) fn local_eligibility_survives_transfer_without_initializer_links() {
    let source = "a:null;b:true;n:1;s:\"x\";xs:[1,2];r:{->n:1};u<int32><null>:n;f<int32>:(v<int32>){->v};copy:r.n";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    assert!(checker.locals.is_empty());
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    assert_eq!(reports.eligible, (0..program.locals.len()).collect());
    assert!(!checker.proofs.aliases.is_empty());
    assert!(reports.eligible.contains(&program.functions[0].params[0]));
    assert!(reports.slot_uses.is_empty());
    let parts = reports.parts;
    assert_eq!(
        checker
            .eligible_locals(&program, &reports, Span::default())
            .unwrap(),
        reports.eligible
    );
    assert_eq!(reports.parts, parts);
}

#[test]
pub(crate) fn local_eligibility_excludes_mutation_references_and_foundation_values() {
    let source = "m:@\"memory\";n:=1;p:&n;row:{->inner:{->n:=2};->xs:=[1,2]};copy:row;a:m.heap";
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    for (id, ty) in program.locals.iter().enumerate() {
        if checker.proofs.variable(id) || ty.has_reference() || matches!(ty, Type::Foundation(_)) {
            assert!(!reports.eligible.contains(&id));
        }
    }
    checker.proofs.fields.clear();
    let eligible = checker
        .eligible_locals(&program, &reports, Span::default())
        .unwrap();
    for (id, ty) in program.locals.iter().enumerate() {
        if ty.has_mutable_fields() {
            assert!(!eligible.contains(&id));
        }
    }
}
