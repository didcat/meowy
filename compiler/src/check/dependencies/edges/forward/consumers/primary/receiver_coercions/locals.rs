use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

mod boundaries;
mod stages;

#[test]
pub(crate) fn shared_local_receiver_coercions_link_owned_primaries_with_exact_descriptors() {
    for init in ["{->p;->tag:true}", "p.{->$;->tag:true}"] {
        for stored in [false, true] {
            let prefix = if stored {
                format!("r:{init};alias:((r));")
            } else {
                String::new()
            };
            let input = if stored { "alias" } else { init };
            let source = format!(
                "n:1;p:&n;flag:=true;{prefix}out:({input}).{{a<&int32>:(($));b<&int32><null>:$;|flag|inner:$.{{c<&int32>:($~<{{-><&int32>;tag<boolean>}}> )}};cmp:p!=(($));value:$.tag;copy:{{->$}}}};f<&int32>:(p<&int32>){{{prefix}->({input}).{{->$}}}}"
            );
            let (mut checker, reports) = checked(&source);
            let ops: Vec<_> = checker
                .coercions
                .iter()
                .filter(|(_, op)| op.primary)
                .map(|(&id, op)| (id, op.clone()))
                .collect();
            assert_eq!(ops.len(), 5, "{source}");
            let mut owners = BTreeSet::new();
            for (id, op) in ops {
                let (_, Effect::Coercion(observed)) = &reports.effects[&id] else {
                    panic!()
                };
                assert_eq!(op.fields, Some(1));
                assert_eq!(observed.fields, op.fields);
                assert!(matches!(op.source, Some(Shape::SharedScalar(_))));
                assert!(observed.projected && observed.result);
                assert_eq!(observed.operation, op.kind == CoercionKind::Convert);
                let port = Port::Projection { point: id, step: 0 };
                let (owner, slot) = reports.slot_uses[&port];
                assert_eq!((owner, slot.index), (op.owner, 0));
                let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(layout.len(), op.fields.unwrap() + 1);
                assert_eq!(op.source, Some(layout[0].shape));
                assert_eq!(
                    reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                    Sources::Unknown
                );
                assert_eq!(
                    checker
                        .primary_effect_inputs(
                            &reports,
                            id,
                            op.owner,
                            &reports.effects[&id].1,
                            Span::default()
                        )
                        .unwrap(),
                    [Some(op.input), None]
                );
                owners.insert(op.owner);
            }
            assert_eq!(owners, BTreeSet::from([0, 1]));
            assert_eq!(reports.field_results.len(), 1);
            assert_eq!(
                reports
                    .slot_uses
                    .keys()
                    .filter(|port| matches!(port, Port::Emission(_)))
                    .count(),
                2
            );
            assert!(
                checker
                    .binaries
                    .values()
                    .all(|op| op.plan.primary == [false; 2])
            );
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
        }
    }
}

#[test]
pub(crate) fn shared_local_receiver_coercions_keep_binary_ownership_and_receiver_copy_boundaries() {
    for tail in [
        "out:r.{cmp:$==p}",
        "out:r.{copy:$;v<&int32>:copy}",
        "out:r.{copy:$;nested:copy.{v<&int32>:$}}",
        "out:(&r).{v<&int32>:*$}",
    ] {
        let source = format!("n:1;p:&n;r:{{->p;->tag:true}};{tail}");
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    for source in [
        "n:1;p:&n;r:={->p;->tag:true};out:r.{copy<&int32>:$}",
        "n:1;p:&n;r:{->p;->tag:=true};out:r.{copy<&int32>:$}",
        "f<null>:(r<{-><&int32>;tag<boolean>}>){out:r.{copy<&int32>:$}}",
        "f<{-><&int32>;tag<boolean>}>:(p<&int32>){->p;->tag:true};n:1;out:f(&n).{copy<&int32>:$}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (checker, reports) =
        checked("n:1;p:&n;r:{->p;->tag:true};out:r.{a<{-><&int32>;tag<boolean>}>:$;b:$==p}");
    assert!(reports.slot_uses.is_empty());
    assert!(
        checker
            .binaries
            .values()
            .any(|op| op.plan.primary == [true, false])
    );
    assert!(checker.coercions.values().all(|op| !op.primary));
}
