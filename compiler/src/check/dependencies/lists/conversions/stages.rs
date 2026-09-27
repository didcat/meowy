use super::*;
use crate::check::dependencies::lists::tests::check;

#[test]
pub(crate) fn contextual_list_stages_put_primary_and_conversion_before_later_effects() {
    let source = "<T>:<int32><boolean>;<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;g<boolean>:(){->true};f:(v<R><S>){|v<R>|xs<T[2]><string[2]>:[v,{n:g();->n}]}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, inputs) = checker.list_inputs.first_key_value().unwrap();
    let key = SequenceSource::Expr(id);
    let primary = Port::Projection { point: id, step: 0 };
    let convert = Port::Conversion { point: id, part: 0 };
    assert!(inputs[0].primary);
    assert_eq!(inputs[0].kind, CoercionKind::Convert);
    assert_eq!(inputs[1].kind, CoercionKind::Forward);
    assert_eq!(checker.invocations.len(), 1);
    assert_ne!(checker.sequences[&key].owner, 0);
    assert_eq!(
        checker.sequences[&key].edges,
        [Edge::new(
            convert,
            Port::Entry(inputs[1].point),
            Route::Next
        )]
    );
    assert_eq!(
        checker.endpoints[&key],
        [
            Edge::new(Port::Entry(id), Port::Entry(inputs[0].point), Route::Next),
            Edge::new(Port::Normal(inputs[0].point), primary, Route::Next),
            Edge::new(primary, convert, Route::Next),
            Edge::new(
                Port::Normal(inputs[1].point),
                Port::Operation(id),
                Route::Next
            ),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ]
    );
}

#[test]
pub(crate) fn contextual_list_stages_keep_deferred_source_order_and_existing_conversions() {
    let source = "<T>:<uint8><boolean>;<U>:<uint16><boolean>;f:(v<uint8><uint16>){|v<uint8>|xs<T[3]><U[3]>:[1,v,3]}";
    let checker = check(source);
    let (&id, inputs) = checker.list_inputs.first_key_value().unwrap();
    let key = SequenceSource::Expr(id);
    assert!(inputs[0].point > inputs[1].point);
    assert_eq!(
        checker.sequences[&key].edges,
        [
            Edge::new(
                Port::Normal(inputs[0].point),
                Port::Entry(inputs[1].point),
                Route::Next
            ),
            Edge::new(
                Port::Conversion { point: id, part: 1 },
                Port::Entry(inputs[2].point),
                Route::Next
            ),
        ]
    );
    let stages: Vec<_> = checker.endpoints[&key]
        .iter()
        .filter(|edge| matches!(edge.to, Port::Conversion { .. } | Port::Projection { .. }))
        .collect();
    assert_eq!(stages.len(), 1);
    assert_eq!(stages[0].to, Port::Conversion { point: id, part: 1 });
}

#[test]
pub(crate) fn contextual_list_stages_stop_before_suffixes_and_construction() {
    for (source, primary) in [
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs<int32[2]><uint8[2]>:[stop(),{x:300;->x}]",
            false,
        ),
        (
            "<R>:<{-><never>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[2]><string[2]>:[v,{x:1;->x}]}",
            true,
        ),
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let (&id, inputs) = checker.list_inputs.first_key_value().unwrap();
        let key = SequenceSource::Expr(id);
        assert_eq!(inputs.len(), 2);
        assert!(checker.sequences[&key].edges.is_empty());
        let edges = &checker.endpoints[&key];
        assert_eq!(edges.len(), if primary { 2 } else { 1 });
        assert!(
            !edges
                .iter()
                .any(|edge| edge.to == Port::Entry(inputs[1].point)
                    || edge.to == Port::Operation(id)
                    || edge.to == Port::Normal(id))
        );
        if primary {
            assert_eq!(
                edges[1],
                Edge::new(
                    Port::Normal(inputs[0].point),
                    Port::Projection { point: id, step: 0 },
                    Route::Next
                )
            );
        }
    }
}

#[test]
pub(crate) fn contextual_list_stages_retain_completed_prefixes_before_stopped_items() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};<T>:<int32><boolean>;f:(v<int32><string>){|v<int32>|xs<T[3]><string[3]>:[v,stop(),{x:true;->x}]}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, inputs) = checker.list_inputs.first_key_value().unwrap();
    let key = SequenceSource::Expr(id);
    let convert = Port::Conversion { point: id, part: 0 };
    assert_eq!(inputs[0].kind, CoercionKind::Convert);
    assert_eq!(inputs[1].kind, CoercionKind::Stopped);
    assert_eq!(
        checker.sequences[&key].edges,
        [Edge::new(
            convert,
            Port::Entry(inputs[1].point),
            Route::Next
        )]
    );
    assert_eq!(
        checker.endpoints[&key],
        [
            Edge::new(Port::Entry(id), Port::Entry(inputs[0].point), Route::Next),
            Edge::new(Port::Normal(inputs[0].point), convert, Route::Next),
        ]
    );
}

#[test]
pub(crate) fn contextual_list_stages_publish_inputs_sequences_and_endpoints_atomically() {
    let source = "f:(v<int32><string>){|v<int32>|xs<int32[1]><string[1]>:[v]}";
    let mut checker = check(source);
    let (&id, inputs) = checker.list_inputs.first_key_value().unwrap();
    let inputs = inputs.clone();
    let key = SequenceSource::Expr(id);
    checker.owner = checker.points[id].owner;
    let counts = (checker.sequence_edges, checker.endpoint_edges);
    checker
        .contextual_list_sequence(id, inputs.clone(), true, Span::default())
        .unwrap();
    assert_eq!((checker.sequence_edges, checker.endpoint_edges), counts);
    let mut changed = inputs.clone();
    changed[0].primary = true;
    assert!(
        checker
            .contextual_list_sequence(id, changed, true, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.list_inputs[&id], inputs);
    checker.list_inputs.remove(&id);
    checker.sequence_edges -= checker.sequences.remove(&key).unwrap().edges.len();
    checker.endpoint_edges -= checker.endpoints.remove(&key).unwrap().len();
    let counts = (checker.sequence_edges, checker.endpoint_edges);
    for (items, normal) in [(vec![inputs[0], inputs[0]], true), (inputs.clone(), false)] {
        assert!(
            checker
                .contextual_list_sequence(id, items, normal, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(!checker.list_inputs.contains_key(&id));
        assert!(!checker.sequences.contains_key(&key));
        assert!(!checker.endpoints.contains_key(&key));
    }
    checker.index_edges = crate::check::dependencies::edges::MAX_EDGES;
    assert!(
        checker
            .contextual_list_sequence(id, inputs, true, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(!checker.list_inputs.contains_key(&id));
    assert!(!checker.sequences.contains_key(&key));
    assert!(!checker.endpoints.contains_key(&key));
    assert_eq!((checker.sequence_edges, checker.endpoint_edges), counts);
}
