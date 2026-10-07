use super::*;
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn receiver_output_primaries_keep_sparse_visits_source_results_and_initialization_independent()
 {
    let (mut checker, mut reports) =
        checked("d:@\"debug\";r:3.{->$;->tag:true};out:r.{d.panic(\"a{$}b{$}c\")}");
    let id = *checker.outputs.keys().next().unwrap();
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 2);
    let slot = expected.values().next().unwrap().1;
    let source = reports.results[&slot.block].1.dispatch.unwrap();
    let receiver = *checker
        .dispatch_ops
        .iter()
        .find(|(_, op)| matches!(op.receiver, Shape::Record { .. }))
        .unwrap()
        .0;
    let Effect::Output(full) = reports.effects[&id].1.clone() else {
        panic!()
    };
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
                for (projected, written, terminal, prefix) in [
                    ([true, false], false, false, false),
                    ([false, true], false, false, false),
                    ([true; 2], false, false, false),
                    ([true; 2], true, true, true),
                    ([true, false], false, true, false),
                    ([false, true], true, false, true),
                    ([false; 2], true, false, false),
                    ([false; 2], false, true, false),
                    ([false; 2], false, false, true),
                ] {
                    let mut output = full.clone();
                    output.prefix = prefix;
                    output.terminal = terminal;
                    output.parts.retain(|&part, row| {
                        let step = match part {
                            1 => 0,
                            3 => 1,
                            _ => return false,
                        };
                        row.projection = projected[step];
                        row.output = written;
                        row.projection || row.output
                    });
                    reports.effects.insert(id, (0, Effect::Output(output)));
                    reports.index.operations.remove(&id);
                    if terminal {
                        reports.index.operations.insert(id, 0);
                    }
                    let selected = expected
                        .iter()
                        .filter_map(|(&port, &value)| {
                            let Port::Projection { step, .. } = port else {
                                panic!()
                            };
                            (projected[(step - 1) / 2] && published && initialized)
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
    reports.effects.insert(id, (0, Effect::Output(full)));
    reports.index.operations.remove(&id);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn receiver_output_primaries_keep_prefix_links_before_stopped_parts_and_bodies() {
    for method in ["print", "panic"] {
        let source = format!(
            "d:@\"debug\";r:3.{{->$;->tag:true}};out:r.{{d.{method}(\"a{{$}}b{{d.panic(\"stop\")}}tail{{$}}\")}}"
        );
        let (mut checker, reports) = checked(&source);
        let (&id, op) = checker
            .outputs
            .iter()
            .find(|(_, op)| op.parts.len() > 1)
            .unwrap();
        let (_, Effect::Output(output)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(op.stopped, Some(3));
        assert_eq!(output.prefix, method == "panic");
        assert!(!output.terminal && !output.parts.contains_key(&5));
        assert!(op.parts[5].unwrap().source.is_some());
        assert_eq!(reports.slot_uses.len(), 1);
        assert!(
            reports
                .slot_uses
                .contains_key(&Port::Projection { point: id, step: 1 })
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
    }
    let (_, reports) =
        checked("d:@\"debug\";r:{->3;->tag:true};out:r.{d.panic(\"stop\");d.print($)}");
    assert!(reports.slot_uses.is_empty());
}

#[test]
pub(crate) fn receiver_output_primaries_keep_opaque_inputs_inner_coercions_and_candidate_histories()
{
    for source in [
        "r:={->3;->tag:true};out:r.{d.print($)}",
        "r:{->3;->tag:=true};out:r.{d.print($)}",
        "r:{->3;->tag:true};p:&r;out:(*p).{d.print($)}",
        "get<{-><int32>;tag<boolean>}>:(){->3;->tag:true};out:get().{d.print($)}",
        "f<null>:(r<{-><int32>;tag<boolean>}>){out:r.{d.print($)}}",
        "f<null>:(r<{-><never>;tag<boolean>}>){out:r.{d.print($)}}",
        "n<int32><null>:1;r:{->n;->tag:true};out:r.{d.print($)}",
    ] {
        let (_, reports) = checked(&format!("d:@\"debug\";{source}"));
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (checker, reports) =
        checked("d:@\"debug\";r:{->3;->tag:true};out:r.{d.print(-$);xs<int32[1]>:[$];whole:{->$}}");
    assert_eq!(reports.slot_uses.len(), 2);
    for port in reports.slot_uses.keys() {
        let Port::Projection { point, step: 0 } = port else {
            panic!()
        };
        assert!(matches!(reports.effects[point].1, Effect::Coercion(_)));
        assert!(!checker.outputs.contains_key(point));
    }
    for (init, count) in [
        ("{->tag:true}", 0),
        ("{flag:=false;|flag|->3;|!flag|->4;->tag:true}", 2),
    ] {
        let (_, reports) = checked(&format!("d:@\"debug\";r:{init};out:r.{{d.print($)}}"));
        assert_eq!(reports.slot_uses.len(), 1);
        let slot = reports.slot_uses.values().next().unwrap().1;
        let Sources::Candidates(values) =
            &reports.results[&slot.block].1.slots.as_ref().unwrap()[0]
        else {
            panic!()
        };
        assert_eq!(values.len(), count);
    }
}
