use super::{super::tests::checked, *};

#[test]
pub(crate) fn dispatch_validation_keeps_receiver_binding_empty_forward_and_composed_bodies() {
    for source in [
        "v:3.{->$}",
        "v:3.{}",
        "n:1;v:(&n).{->*$}",
        "v:3.{f<()->int32>;f<int32>:(){->1};->$}",
        "v<{x<int32>;y<int32>}>:{->((3.{->x:$}));->y:4}",
        "g<int32>:(){->3};f<{x<int32>}>:(){->g().{->x:$}}",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        let owner = op.owner;
        assert!(checker.locals.is_empty());
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(
                checker
                    .validate_dispatch_effect(&reports, owner, port, Span::default())
                    .unwrap()
            );
        }
    }
}

#[test]
pub(crate) fn dispatch_validation_distinguishes_stopped_receivers_and_bodies() {
    for (tail, init) in [("stop().{}", false), ("3.{stop()}", true)] {
        let source = format!("d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};v:{tail}");
        let (mut checker, reports) = checked(&source, false);
        let id = *checker.dispatch_ops.first_key_value().unwrap().0;
        assert_eq!(
            checker
                .validate_dispatch_effect(&reports, 0, Port::Operation(id), Span::default())
                .is_ok(),
            init
        );
        assert!(
            checker
                .validate_dispatch_effect(&reports, 0, Port::Normal(id), Span::default())
                .is_err()
        );
    }
}

#[test]
pub(crate) fn dispatch_validation_rejects_corrupt_roots_bindings_sequences_and_edges() {
    for normal in [false, true] {
        for fault in 0..39 {
            let (mut checker, mut reports) = checked("v:3.{n:1;->$}", false);
            let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
            let input = op.input;
            let block = op.block;
            let key = SequenceSource::Block(block);
            let next = checker.sequences[&key].items[1].unwrap();
            let site = checker.points[next].site.unwrap();
            let op = checker.dispatch_ops.get_mut(&id).unwrap();
            match fault {
                0 => op.owner += 1,
                1 => op.input = id,
                2 => op.input = usize::MAX,
                3 => op.local = reports.locals,
                4 => op.block = usize::MAX,
                5 => op.input_normal = false,
                6 => op.normal = false,
                7 => op.span = Span::default(),
                8 => checker.points[id].owner += 1,
                9 => checker.points[id].complete = false,
                10 => checker.points[id].kind = PointKind::Stmt,
                11 => checker.points[input].owner += 1,
                12 => checker.points[input].parent = None,
                13 => checker.points[input].block = None,
                14 => checker.points[input].complete = false,
                15 => checker.points[input].kind = PointKind::Read,
                16 => checker.bodies.get_mut(&block).unwrap().owner += 1,
                17 => checker.bodies.get_mut(&block).unwrap().parent = None,
                18 => {
                    checker.bodies.get_mut(&block).unwrap().facts[0].0 = Fact::Bind(reports.locals)
                }
                19 => checker.bodies.get_mut(&block).unwrap().storage[0] = None,
                20 => checker.bodies.get_mut(&block).unwrap().sources[0] = Some(input),
                21 => {
                    checker.proofs.dispatches.remove(&block);
                }
                22 => {
                    checker.proofs.receivers.remove(&op.local);
                }
                23 => checker.sequences.get_mut(&key).unwrap().owner += 1,
                24 => checker.sequences.get_mut(&key).unwrap().items[0] = Some(next),
                25 => checker.sequences.get_mut(&key).unwrap().items[1] = None,
                26 => checker.points[next].block = None,
                27 => checker.sites.get_mut(&site).unwrap().point = None,
                28 => checker.sites.get_mut(&site).unwrap().complete = false,
                29 => checker.endpoints.get_mut(&key).unwrap().clear(),
                30 => checker.endpoints.get_mut(&key).unwrap()[0].route = Route::Next,
                31 => op.edges.clear(),
                32 => op.edges[0].route = Route::Result,
                33 => op.edges[3].to = Port::BlockNormal(block),
                34 => {
                    op.edges.pop();
                }
                35 => op.edges.push(op.edges[0]),
                36 => {
                    reports.index.operations.remove(&id);
                }
                37 => {
                    reports.index.operations.insert(id, 9);
                }
                38 => {
                    checker.endpoints.insert(SequenceSource::Expr(id), vec![]);
                }
                _ => unreachable!(),
            }
            let port = if normal {
                Port::Normal(id)
            } else {
                Port::Operation(id)
            };
            assert!(
                checker
                    .validate_dispatch_effect(&reports, 0, port, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "fault {fault}"
            );
        }
    }
}
