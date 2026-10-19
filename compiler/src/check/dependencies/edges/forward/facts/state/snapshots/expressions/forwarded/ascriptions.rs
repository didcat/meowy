use super::super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_ascription_values_preserve_parameter_versions_and_operation_storage() {
    let (checker, program, _, snapshots) =
        capture("(7)~<int32>;f:(n<uint8>){v<uint8>:(((n)~<uint8>))}");
    for (&point, op) in &checker.typed_ops {
        assert_eq!(
            snapshots.expressions[&point],
            snapshots.expressions[&op.input]
        );
        assert!(snapshots.expressions[&point].value.is_some());
        assert_eq!(
            snapshots.before(Port::Operation(point)),
            snapshots.after(Port::Operation(point))
        );
        assert_eq!(
            snapshots.after(Port::Operation(point)),
            snapshots.before(Port::Normal(point))
        );
    }
    let binding = checker
        .operations
        .values()
        .find(|op| op.owner == 1)
        .unwrap();
    let value = snapshots.expressions[&binding.input.unwrap()]
        .value
        .as_ref()
        .unwrap();
    assert_eq!(value.version.unwrap().local, program.functions[0].params[0]);
    assert_eq!(value.origin, Some(Origin::Parameter(0, 0)));
}
