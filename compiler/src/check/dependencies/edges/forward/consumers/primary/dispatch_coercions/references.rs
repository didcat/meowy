use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn shared_dispatch_coercions_link_exact_primaries_across_contexts_and_owners() {
    for (ty, value) in [
        ("int32", "1"),
        ("uint64", "1"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        let body = format!("(p.{{->$;->tag:true}}~<{{-><&{ty}>;tag<boolean>}}>)");
        let source = format!(
            "n<{ty}>:{value};p:&n;a<&{ty}>:{body};b<&{ty}><null>:{body};x:{body}==p;f<&{ty}>:(p<&{ty}>){{->{body}}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 4, "{source}");
        assert!(reports.slot_uses.values().any(|(owner, _)| *owner != 0));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        for (&port, &(owner, slot)) in &reports.slot_uses {
            let Port::Projection { point, step: 0 } = port else {
                panic!()
            };
            let (_, Effect::Coercion(op)) = &reports.effects[&point] else {
                panic!()
            };
            assert!(op.primary && op.projected && op.result);
            assert_eq!(op.operation, op.op == CoercionKind::Convert);
            assert!(matches!(op.source, Some(Shape::SharedScalar(_))));
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(op.source, Some(slots[0].shape));
            assert_eq!(slot.index, 0);
            let result = &reports.results[&slot.block].1;
            let dispatch = result.dispatch.unwrap();
            assert_eq!(checker.dispatch_ops[&dispatch].owner, owner);
            assert_eq!(result.slots.as_ref().unwrap()[0], Sources::Unknown);
            assert_eq!(
                checker
                    .forward_coercion_input(&reports, point, owner, Span::default())
                    .unwrap(),
                None
            );
        }
        for (&id, _) in &checker.binaries {
            let (_, Effect::Binary(op)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(op.projected, [false; 2]);
            assert!(op.operation && op.result);
        }
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_dispatch_coercions_keep_each_stage_and_registration_independent() {
    for target in ["&int32", "&int32><null"] {
        let source =
            format!("n:1;p:&n;x<{target}>:(p.{{->$;->tag:true}}~<{{-><&int32>;tag<boolean>}}>)");
        let (mut checker, mut reports) = checked(&source);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary)
            .unwrap()
            .0;
        let dispatch = *checker.dispatch_ops.keys().next().unwrap();
        let expected = reports.slot_uses.clone();
        assert_eq!(expected.len(), 1);
        let convert = checker.coercions[&id].kind == CoercionKind::Convert;
        for initialized in [false, true] {
            for completed in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.initialized = initialized;
                op.result = completed;
                for (projected, operation, result) in [
                    (true, false, false),
                    (true, convert, true),
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
                        if projected && completed {
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
pub(crate) fn shared_dispatch_coercions_keep_local_receiver_and_stopped_boundaries() {
    for source in [
        "n:1;p:&n;r:p.{->$;->tag:true};x<&int32>:r",
        "n:1;p:&n;r:=p.{->$;->tag:true};x<&int32>:r",
        "n:1;p:&n;r:p.{->$;->tag:true};q:&r;x<&int32>:*q",
        "n:1;p:&n;x:p.{->$;->tag:true}.{x<&int32>:$}",
        "n:[1];p:&n;x<&int32[1]>:(p.{->$;->tag:true}~<{-><&int32[1]>;tag<boolean>}>)",
        "d:@\"debug\";n:1;p:&n;x<&int32>:(p.{->$;->tag:true;d.panic(\"stop\")}~<{-><&int32>;tag<boolean>}>)",
        "d:@\"debug\";n:1;p:&n;x<&int32><null>:(p.{->$;->tag:true;d.panic(\"stop\")}~<{-><&int32>;tag<boolean>}>)",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let source = "flag:=false;n:1;m:2;p:&n;q:&m;x<&int32>:(p.{|flag|->$;|!flag|->q;->tag:true}~<{-><&int32>;tag<boolean>}>)";
    let (_, reports) = checked(source);
    assert_eq!(reports.slot_uses.len(), 1);
    let slot = reports.slot_uses.values().next().unwrap().1;
    assert_eq!(
        reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
        Sources::Unknown
    );
}
