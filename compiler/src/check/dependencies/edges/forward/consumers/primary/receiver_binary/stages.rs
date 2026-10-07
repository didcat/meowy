use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn receiver_binary_primaries_keep_each_projection_source_and_receiver_independent() {
    let (mut checker, mut reports) = checked(
        "a:3.{->$;->tag:true};b:4.{->$;->tag:false};out:a.{left:$;inner:b.{right:$;x:left+right}}",
    );
    let id = *checker.binaries.keys().next().unwrap();
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 2);
    let sources = [0, 1].map(|step| {
        let slot = expected[&Port::Projection { point: id, step }].1;
        reports.results[&slot.block].1.dispatch.unwrap()
    });
    let receivers: Vec<_> = checker
        .dispatch_ops
        .iter()
        .filter_map(|(&id, op)| matches!(op.receiver, Shape::Record { .. }).then_some(id))
        .collect();
    assert_eq!(receivers.len(), 2);
    let flags = [[false; 2], [true, false], [false, true], [true; 2]];
    for published in flags {
        for (step, source) in sources.into_iter().enumerate() {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                panic!()
            };
            op.result = published[step];
            op.initialized = !published[step];
        }
        for initialized in flags {
            for completed in flags {
                for (step, receiver) in receivers.iter().enumerate() {
                    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(receiver).unwrap()
                    else {
                        panic!()
                    };
                    op.initialized = initialized[step];
                    op.result = completed[step];
                }
                for (projected, operation, result) in [
                    ([true, false], false, false),
                    ([false, true], false, false),
                    ([true; 2], false, false),
                    ([true; 2], true, true),
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
                            (projected[step] && published[step] && initialized[step])
                                .then_some((port, value))
                        })
                        .collect::<Uses>();
                    assert_eq!(
                        checker.slot_uses(&reports, Span::default()).unwrap(),
                        selected
                    );
                }
            }
        }
    }
    reports.index.operations.remove(&id);
    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.projected = [true; 2];
    op.operation = false;
    op.result = false;
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    for (operation, result) in [(true, false), (false, true)] {
        let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.operation = operation;
        op.result = result;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn receiver_binary_primaries_keep_left_projection_before_stopped_right_inputs() {
    let (mut checker, reports) =
        checked("d:@\"debug\";r:{->3;->tag:true};out:r.{x:$+d.panic(\"stop\")}");
    let id = *checker.binaries.keys().next().unwrap();
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
    for source in [
        "d:@\"debug\";r:{->3;->tag:true};out:r.{x:d.panic(\"stop\")+$}",
        "d:@\"debug\";r:{->3;->tag:true};out:r.{d.panic(\"stop\");x:$+1}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty());
    }
}

#[test]
pub(crate) fn receiver_binary_primaries_preserve_empty_multiple_and_opaque_histories() {
    for (source, count) in [
        ("r:{->tag:true};out:r.{x:$==null}", 0),
        (
            "flag:=false;r:{|flag|->3;|!flag|->4;->tag:true};out:r.{x:$+1}",
            2,
        ),
    ] {
        let (_, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), 1);
        let slot = reports.slot_uses.values().next().unwrap().1;
        let Sources::Candidates(values) =
            &reports.results[&slot.block].1.slots.as_ref().unwrap()[0]
        else {
            panic!()
        };
        assert_eq!(values.len(), count);
    }
    for source in [
        "r:{->3;->tag:true};out:r.{x:$==r}",
        "r:={->3;->tag:true};out:r.{x:$+1}",
        "r:{->3;->tag:=true};out:r.{x:$+1}",
        "r:{->3;->tag:true};p:&r;out:(*p).{x:$+1}",
        "get<{-><int32>;tag<boolean>}>:(){->3;->tag:true};out:get().{x:$+1}",
        "f:(r<{-><int32>;tag<boolean>}>){out:r.{x:$+1}}",
        "a:1;r:{->&a;->tag:true};out:r.{x:$==&a}",
        "r:{->[1];->tag:true};out:r.{x:$==r}",
        "n<int32><null>:1;r:{->n;->tag:true};out:r.{x:$==n}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (_, reports) = checked("r:{->3;->tag:true};out:r.{x:$+1;copy:{->x}}");
    assert!(
        reports
            .direct_sources
            .values()
            .all(|(_, direct)| direct.source.is_none()
                && direct.block.is_none()
                && direct.dispatch.is_none())
    );
}
