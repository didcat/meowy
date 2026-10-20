use super::super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_narrowing_values_carry_parameter_versions_through_groups_and_typed_inputs() {
    let (checker, program, _, snapshots) =
        capture("f:(n<uint8>){v<uint8>:((n))};g:(b<boolean><null>){((b))}");
    for (&point, op) in &checker.narrowings {
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        assert_eq!(
            snapshots.expressions[&point],
            snapshots.expressions[&op.input]
        );
        assert_eq!(
            value.version.unwrap().local,
            program.functions[op.owner - 1].params[0]
        );
        assert_eq!(value.origin, Some(Origin::Parameter(op.owner - 1, 0)));
    }
    let binding = checker
        .operations
        .values()
        .find(|op| op.owner == 1)
        .unwrap();
    assert_eq!(
        snapshots.expressions[&binding.input.unwrap()]
            .value
            .as_ref()
            .unwrap()
            .version
            .unwrap()
            .local,
        program.functions[0].params[0]
    );
    let (checker, _, _, snapshots) = capture("g:(){};g();x:=1;((x))");
    for point in checker.narrowings.keys().chain(checker.group_inputs.keys()) {
        assert!(snapshots.expressions[point].value.is_none());
    }
}
