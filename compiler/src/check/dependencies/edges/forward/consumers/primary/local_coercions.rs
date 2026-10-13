use super::{super::tests::checked, *};
use crate::check::dependencies::{CoercionKind, edges::forward::results::Sources};
use std::collections::BTreeSet;

mod boundaries;

#[test]
pub(crate) fn shared_local_coercions_link_original_record_primaries_through_copies() {
    for init in [
        "{->p;->a:true;->z:3}",
        "p.{->$;->a:true;->z:3}",
        "3.{->p;->a:true;->z:3}",
    ] {
        let source = format!(
            "n:1;p:&n;r:{init};alias:((r));a<&int32>:alias;b<&int32><null>:(r~<{{-><&int32>;a<boolean>;z<int32>}}>);f<&int32>:(p<&int32>){{r:{init};->r}}"
        );
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 3);
        let mut owners = BTreeSet::new();
        for (&port, &(owner, slot)) in &reports.slot_uses {
            let Port::Projection { point: id, step: 0 } = port else {
                panic!()
            };
            let (_, Effect::Coercion(op)) = &reports.effects[&id] else {
                panic!()
            };
            assert!(op.primary && op.projected);
            assert_eq!(op.fields, Some(2));
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(slot.index, 0);
            assert_eq!(slots.len(), op.fields.unwrap() + 1);
            assert_eq!(op.source, Some(slots[0].shape));
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
            owners.insert(owner);
        }
        assert_eq!(owners, BTreeSet::from([0, 1]));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_coercions_preserve_source_and_projection_conversion_result_stages() {
    for target in ["&int32", "&int32><null"] {
        let source = format!("n:1;p:&n;r:p.{{->$;->tag:true}};copy<{target}>:r");
        let (mut checker, mut reports) = checked(&source);
        let (&port, &(_, slot)) = reports.slot_uses.first_key_value().unwrap();
        let Port::Projection { point: id, step: 0 } = port else {
            panic!()
        };
        let source = reports.results[&slot.block].1.dispatch.unwrap();
        let expected = reports.slot_uses.clone();
        let convert = checker.coercions[&id].kind == CoercionKind::Convert;
        for initialized in [false, true] {
            for published in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.initialized = initialized;
                op.result = published;
                for (projected, operation, result) in [
                    (true, false, false),
                    (true, convert, true),
                    (false, convert, true),
                    (false, false, true),
                ] {
                    let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.projected = projected;
                    op.operation = operation;
                    op.result = result;
                    if !operation && !result {
                        reports.index.operations.remove(&id);
                    } else if convert {
                        reports.index.operations.insert(id, 0);
                    }
                    assert_eq!(
                        checker.slot_uses(&reports, Span::default()).unwrap(),
                        if projected && published {
                            expected.clone()
                        } else {
                            Uses::new()
                        }
                    );
                }
            }
        }
        if convert {
            reports.index.operations.remove(&id);
            assert!(
                checker
                    .slot_uses(&reports, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
        }
        assert_eq!(reports.slot_uses, expected);
    }
}

#[test]
pub(crate) fn shared_local_coercions_keep_stopped_prefixes_and_binary_receiver_boundaries() {
    let (_, reports) = checked(
        "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true};copy<&int32>:r;d.panic(\"stop\");later<&int32>:r",
    );
    assert_eq!(reports.slot_uses.len(), 1);
    for tail in [
        "out:p!=r",
        "out:r.{copy<&int32>:$}",
        "out:r.{alias:$;copy<&int32>:alias}",
    ] {
        let source = format!("n:1;p:&n;r:{{->p;->tag:true}};{tail}");
        crate::compile(&source).unwrap();
        let (_, reports) = checked(&source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
