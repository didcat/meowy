use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::Sources;

mod boundaries;

#[test]
pub(crate) fn dispatch_binary_primaries_preserve_operand_positions_types_and_owners() {
    for source in [
        "a:3.{->$;->tag:true};b:4.{->$;->tag:false};x:((a))+((b));f<int32>:(){r:5.{->$;->tag:true};->r+1}",
        "r:3.{->$;->tag:true};a:1+r;b:r-1;c:r<4;d:3==r",
        "r:1.5.{->$;->tag:true};x:r/2.0",
        "r:\"cat\".{->$;->tag:true};x:r<\"dog\"",
        "r:true.{->$;->tag:true};x:r==true",
        "r:3.{->tag:true};x:r==null",
    ] {
        let (mut checker, reports) = checked(source);
        let mut count = 0;
        for (&id, op) in &checker.binaries {
            let (_, Effect::Binary(binary)) = &reports.effects[&id] else {
                panic!()
            };
            for (step, projected) in binary.projected.into_iter().enumerate() {
                let port = Port::Projection { point: id, step };
                if !projected {
                    assert!(!reports.slot_uses.contains_key(&port));
                    continue;
                }
                count += 1;
                let (owner, slot) = reports.slot_uses[&port];
                assert_eq!((owner, slot.index), (op.owner, 0));
                let BinaryClass::Scalar(ty) = binary.types.inputs[step] else {
                    panic!()
                };
                let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(slots[0].shape, Shape::Scalar(ty));
                let row = &reports.results[&slot.block].1;
                assert!(row.dispatch.is_some() && row.consumer.is_none());
            }
        }
        assert!(count > 0);
        assert_eq!(count, reports.slot_uses.keys().filter(|port| matches!(port,
            Port::Projection { point, .. } if matches!(reports.effects[point].1, Effect::Binary(_)))).count());
        assert!(reports.slot_uses.keys().all(|port| matches!(port,
            Port::Projection { point, .. } if matches!(reports.effects[point].1, Effect::Binary(_) | Effect::Coercion(_)))));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn dispatch_binary_primaries_keep_each_projection_and_source_result_independent() {
    let (mut checker, mut reports) = checked("a:3.{->$;->tag:true};b:4.{->$;->tag:false};x:a+b");
    let id = *checker.binaries.keys().next().unwrap();
    let dispatches: Vec<_> = checker.dispatch_ops.keys().copied().collect();
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 2);
    for completed in [[false, false], [true, false], [false, true], [true, true]] {
        for (step, &dispatch) in dispatches.iter().enumerate() {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.result = completed[step];
            op.initialized = !completed[step];
        }
        for (projected, operation, result) in [
            ([true, false], false, false),
            ([false, true], false, false),
            ([true, true], true, true),
            ([false; 2], true, false),
            ([false; 2], false, true),
        ] {
            let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.projected = projected;
            op.operation = operation;
            op.result = result;
            let selected = expected
                .iter()
                .filter_map(|(&port, &value)| {
                    let Port::Projection { step, .. } = port else {
                        panic!()
                    };
                    (projected[step] && completed[step]).then_some((port, value))
                })
                .collect::<Uses>();
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                selected
            );
        }
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn dispatch_binary_primaries_keep_left_projection_before_stopped_right_input() {
    let (mut checker, reports) = checked("d:@\"debug\";r:3.{->$;->tag:true};x:r+d.panic(\"stop\")");
    let (&id, _) = checker.binaries.first_key_value().unwrap();
    let (_, Effect::Binary(op)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(op.projected, [true, false]);
    assert!(!op.operation && !op.result);
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(
        reports
            .slot_uses
            .contains_key(&Port::Projection { point: id, step: 0 })
    );
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    let (_, reports) =
        checked("d:@\"debug\";stop<never>:(){d.panic(\"stop\")};r:3.{->$;->tag:true};x:stop()+r");
    assert!(reports.slot_uses.is_empty());
}

#[test]
pub(crate) fn dispatch_binary_primaries_preserve_empty_multiple_and_opaque_histories() {
    let (_, reports) = checked("r:3.{->tag:true};x:r==null");
    let slot = reports.slot_uses.values().next().unwrap().1;
    assert_eq!(
        reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
        Sources::Candidates(Vec::new())
    );
    let (_, reports) = checked("flag:=false;r:3.{|flag|->$;|!flag|->4;->tag:true};x:r+1");
    let slot = reports.slot_uses.values().next().unwrap().1;
    assert!(
        matches!(&reports.results[&slot.block].1.slots.as_ref().unwrap()[0], Sources::Candidates(values) if values.len() == 2)
    );
    for source in [
        "a:3.{->$;->tag:true};b:4.{->$;->tag:false};x:a==b",
        "r:=3.{->$;->tag:true};x:r+1",
        "r:3.{->$;->tag:true};p:&r;x:(*p)+1",
        "f<{-><int32>;tag<boolean>}>:(){->3;->tag:true};x:f()+1",
        "a:1;r:a.{->&a;->tag:true};x:r==&a",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (_, reports) = checked("r:3.{->$;->tag:true};x:r+1;out:{->x}");
    assert!(
        reports
            .direct_sources
            .values()
            .all(|(_, direct)| direct.source.is_none()
                && direct.block.is_none()
                && direct.dispatch.is_none())
    );
}
