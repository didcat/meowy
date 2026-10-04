use super::{super::tests::checked, *};
use crate::check::dependencies::TypedKind;

#[test]
pub(crate) fn ascription_consumers_link_fields_primaries_and_composition_through_copies() {
    for (source, count) in [
        ("v:({->n:1}~<{n<int32>}>).n", 1),
        ("r:{->n:1};s:((r))~<{n<int32>}>;v:((s)).n", 1),
        ("r:{->7;->tag:true};s:((r))~<(r<>)>;v:-s", 1),
        ("r:{->7;->tag:true};s:((r))~<(r<>)>;v:{->s}", 2),
        (
            "f<int32>:(){r:{->n:1};s:((r))~<{n<int32>}>;->s.n};v:({->n:2}~<{n<int32>}>).n",
            2,
        ),
        ("r<{n<int32>}>:(({->n:1}~<{n<int32>}>));v:r.n", 1),
    ] {
        let (mut checker, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), count, "{source}");
        let ids: Vec<_> = checker
            .typed_ops
            .iter()
            .map(|(&id, op)| (id, op.owner))
            .collect();
        for (id, owner) in ids {
            let anchor = checker
                .grouped_consumer(&reports, id, owner, Span::default())
                .unwrap()
                .unwrap();
            let (_, block) = reports.consumers[&anchor];
            assert_eq!(checker.bodies[&block].owner, owner);
            assert!(
                reports
                    .slot_uses
                    .values()
                    .any(|&(seen, slot)| seen == owner && slot.block == block)
            );
        }
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
    }
}

#[test]
pub(crate) fn ascription_consumers_preserve_predicate_changed_and_opaque_value_boundaries() {
    for source in [
        "v:({->n:1}<{n<int32>}>)",
        "<U>:<{n<int32>}><null>;v:{->n:1}~<U>",
        "r:{->n:1};p:&r;v:((*p)~<{n<int32>}>).n",
        "f<{n<int32>}>:(){->n:1};v:(f()~<{n<int32>}>).n",
        "f:(r<{n<int32>}>){v:(r~<{n<int32>}>).n}",
        "r:={->n:1};v:(r~<{n<int32>}>).n",
        "d:@\"debug\";v:d.panic(\"stop\")~<int32>",
    ] {
        let (mut checker, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
        let ids: Vec<_> = checker
            .typed_ops
            .iter()
            .map(|(&id, op)| (id, op.owner))
            .collect();
        for (id, owner) in ids {
            assert_eq!(
                checker
                    .grouped_consumer(&reports, id, owner, Span::default())
                    .unwrap(),
                None,
                "{source}"
            );
        }
    }
    let (mut checker, reports) = checked("v:(({->inner:{->n:1}}).inner~<{n<int32>}>).n");
    assert_eq!(reports.slot_uses.len(), 1);
    let (&id, op) = checker.typed_ops.first_key_value().unwrap();
    let owner = op.owner;
    assert_eq!(
        checker
            .grouped_consumer(&reports, id, owner, Span::default())
            .unwrap(),
        None
    );
}

#[test]
pub(crate) fn ascription_consumers_require_results_but_not_operation_observations() {
    for missing in [false, true] {
        let (mut checker, mut reports) = checked("v:({->n:1}~<{n<int32>}>).n");
        let id = *checker.typed_ops.first_key_value().unwrap().0;
        let (_, Effect::Typed(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        assert_eq!(op.op, TypedKind::Ascription);
        op.operation = false;
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        if missing {
            reports.effects.remove(&id);
        } else {
            let (_, Effect::Typed(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.result = false;
        }
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap()
                .is_empty()
        );
    }
}
