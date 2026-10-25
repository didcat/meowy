use super::super::super::capture::tests::capture;
use super::*;
use crate::check::dependencies::CoercionKind;

#[test]
pub(crate) fn scalar_conversion_operations_preserve_storage_and_input_identity() {
    let (checker, _, _, snapshots) = capture("x:=7;v<int32><null>:x;x=9;x");
    let (&point, op) = checker
        .coercions
        .iter()
        .find(|(_, op)| op.scalar.is_some())
        .unwrap();
    let source = snapshots.after(Port::Normal(op.input)).unwrap();
    assert!(matches!(source, View::Known(_)));
    assert_eq!(snapshots.before(Port::Operation(point)), Some(source));
    assert_eq!(snapshots.after(Port::Operation(point)), Some(source));
    assert_eq!(snapshots.after(Port::Normal(point)), Some(source));
    assert_eq!(
        snapshots.expressions[&point],
        snapshots.expressions[&op.input]
    );
    let read = checker.local_reads.keys().next_back().unwrap();
    assert!(snapshots.expressions[read].value.is_some());
}

#[test]
pub(crate) fn scalar_forward_values_reach_typed_binding_inputs_without_changing_storage() {
    let (mut checker, _, _, snapshots) =
        capture("n<uint64>:((18446744073709551615));f:(){v<boolean>:(true)}");
    for (&point, op) in &checker.coercions {
        assert_eq!(op.kind, CoercionKind::Forward);
        assert_eq!(
            snapshots.expressions[&point],
            snapshots.expressions[&op.input]
        );
        assert!(snapshots.expressions[&point].value.is_some());
        assert_eq!(
            snapshots.before(Port::Normal(point)),
            snapshots.after(Port::Normal(op.input))
        );
    }
    let binding = checker
        .operations
        .values()
        .find(|op| op.owner == 0)
        .unwrap();
    assert_eq!(
        snapshots.expressions[&binding.input.unwrap()]
            .value
            .as_ref()
            .unwrap()
            .domain,
        Domain::integer(18446744073709551615, &mut checker.flow, Span::default()).unwrap()
    );
    let Some(View::Known(before)) = snapshots.before(Port::Operation(
        *checker.operations.first_key_value().unwrap().0,
    )) else {
        panic!()
    };
    assert_eq!(
        before.get(binding.storage).unwrap().data,
        Data::Uninitialized
    );
}
