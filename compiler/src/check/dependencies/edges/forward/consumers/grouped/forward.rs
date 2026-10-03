use super::{super::tests::checked, *};
use crate::check::dependencies::{CoercionKind, OperationKind};

#[test]
pub(crate) fn forward_consumers_resolve_typed_initializer_roots_without_local_joins() {
    for source in [
        "r<{n<int32>}>:(({->n:1}));v:r.n",
        "f<int32>:(){r<{n<int32>}>:(({->n:1}));->r.n}",
    ] {
        let (mut checker, reports) = checked(source);
        let op = checker
            .operations
            .values()
            .find(|op| {
                op.kind == OperationKind::Bind
                    && op
                        .input
                        .and_then(|input| checker.coercions.get(&input))
                        .is_some_and(|coercion| {
                            coercion.kind == CoercionKind::Forward && !coercion.primary
                        })
            })
            .unwrap();
        let (root, owner) = (op.input.unwrap(), op.owner);
        assert!(!reports.consumers.contains_key(&root));
        let mut input = root;
        for index in 0..5 {
            if index % 2 == 0 {
                let coercion = &checker.coercions[&input];
                assert_eq!(
                    checker.points[input].span,
                    checker.points[coercion.input].span
                );
                input = coercion.input;
            } else {
                input = checker.group_inputs[&input].input;
            }
        }
        let &(block_owner, block) = reports.consumers.get(&input).unwrap();
        assert_eq!(block_owner, owner);
        assert_eq!(reports.results[&block].1.consumer, Some(input));
        assert_eq!(
            checker
                .grouped_consumer(&reports, root, owner, Span::default())
                .unwrap(),
            Some(input)
        );
        assert!(reports.slot_uses.is_empty());
    }
}
