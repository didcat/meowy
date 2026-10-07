use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn receiver_unary_primaries_keep_projection_ascription_and_receiver_stages_independent()
{
    let (mut checker, mut reports) = checked(SOURCE);
    let id = *checker.unaries.keys().next().unwrap();
    let ascription = *checker.typed_ops.keys().next().unwrap();
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 1);
    let slot = expected.values().next().unwrap().1;
    let source = reports.results[&slot.block].1.dispatch.unwrap();
    let receiver = *checker
        .dispatch_ops
        .iter()
        .find(|(_, op)| matches!(op.receiver, Shape::Record { .. }))
        .unwrap()
        .0;
    for published in [false, true] {
        let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
            panic!()
        };
        op.result = published;
        op.initialized = !published;
        for initialized in [false, true] {
            for completed in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.initialized = initialized;
                op.result = completed;
                for ascribed in [false, true] {
                    let (_, Effect::Typed(op)) = reports.effects.get_mut(&ascription).unwrap()
                    else {
                        panic!()
                    };
                    op.result = ascribed;
                    op.operation = !ascribed;
                    for (projected, operation, result) in [
                        (true, false, false),
                        (true, true, true),
                        (false, true, false),
                        (false, false, true),
                    ] {
                        let (_, Effect::Unary(op)) = reports.effects.get_mut(&id).unwrap() else {
                            panic!()
                        };
                        op.projected = projected;
                        op.operation = operation;
                        op.result = result;
                        assert_eq!(
                            checker.slot_uses(&reports, Span::default()).unwrap(),
                            if projected && ascribed && initialized && published {
                                expected.clone()
                            } else {
                                Uses::new()
                            }
                        );
                    }
                }
            }
        }
    }
    let (_, Effect::Unary(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.projected = true;
    op.operation = false;
    op.result = false;
    reports.index.operations.remove(&id);
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    for (operation, result) in [(true, false), (false, true)] {
        let (_, Effect::Unary(op)) = reports.effects.get_mut(&id).unwrap() else {
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
pub(crate) fn receiver_unary_primaries_keep_earlier_projections_before_stopped_bodies() {
    let prefix = "d:@\"debug\";<R>:<{-><int32>;tag<boolean>}>;r:{->3;->tag:true}";
    let (mut checker, reports) = checked(&format!(
        "{prefix};out:r.{{x:-($~<R>);d.print(x);d.panic(\"stop\")}}"
    ));
    let id = *checker.unaries.keys().next().unwrap();
    let (_, Effect::Unary(op)) = &reports.effects[&id] else {
        panic!()
    };
    assert!(op.projected && op.operation && op.result);
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(
        reports
            .slot_uses
            .contains_key(&Port::Projection { point: id, step: 0 })
    );
    assert!(
        reports
            .effects
            .values()
            .any(|(_, op)| matches!(op, Effect::Dispatch(op) if op.initialized && !op.result))
    );
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    for body in ["d.panic(\"stop\");x:-($~<R>)", "x:-d.panic(\"stop\")"] {
        let (_, reports) = checked(&format!("{prefix};out:r.{{{body}}}"));
        assert!(reports.slot_uses.is_empty());
    }
}

#[test]
pub(crate) fn receiver_unary_primaries_keep_opaque_inputs_multiple_histories_and_arithmetic_results()
 {
    for source in [
        "r:={->3;->tag:true};out:r.{x:-($~<R>)}",
        "r:{->3;->tag:=true};<M>:<(r<>)>;out:r.{x:-($~<M>)}",
        "base:1;r:{->3;->p:&base};<M>:<(r<>)>;out:r.{x:-($~<M>)}",
        "r:{->3;->tag:true};p:&r;out:(*p).{x:-($~<R>)}",
        "get<R>:(){->3;->tag:true};out:get().{x:-($~<R>)}",
        "f:(r<R>){out:r.{x:-($~<R>)}}",
        "<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){out:v.{|$<R>|x:-($~<R>)}}",
    ] {
        let (_, reports) = checked(&format!("<R>:<{{-><int32>;tag<boolean>}}>;{source}"));
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (_, reports) = checked(
        "<R>:<{-><int32>;tag<boolean>}>;flag:=false;r:{|flag|->3;|!flag|->4;->tag:true};out:r.{x:-($~<R>)}",
    );
    assert_eq!(reports.slot_uses.len(), 1);
    let slot = reports.slot_uses.values().next().unwrap().1;
    let Sources::Candidates(values) = &reports.results[&slot.block].1.slots.as_ref().unwrap()[0]
    else {
        panic!()
    };
    assert_eq!(values.len(), 2);
    let (_, reports) =
        checked("<R>:<{-><int32>;tag<boolean>}>;r:{->3;->tag:true};out:r.{x:-($~<R>);copy:{->x}}");
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(
        reports
            .direct_sources
            .values()
            .all(|(_, direct)| direct.source.is_none()
                && direct.block.is_none()
                && direct.dispatch.is_none())
    );
}
