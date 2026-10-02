use super::{super::tests::checked, *};

#[test]
pub(crate) fn dispatch_effects_keep_opaque_successors_without_skipping_to_later_statements() {
    let (mut checker, mut reports) = checked("v:3.{n:1;->$}", false);
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    let key = SequenceSource::Block(op.block);
    let sequence = checker.sequences.get_mut(&key).unwrap();
    let last = sequence.items[2].unwrap();
    sequence.items[1] = None;
    checker.sequence_edges -= sequence.edges.len();
    sequence.edges.clear();
    checker.dispatch_ops.get_mut(&id).unwrap().edges.remove(3);
    checker.dispatch_edges -= 1;
    reports.index = checker.forward_index(Span::default()).unwrap();
    let (root, walk) = reports.entries.get_mut(&0).unwrap();
    *walk = reports
        .index
        .walk(Port::BlockEntry(*root), &mut checker.flow, Span::default())
        .unwrap();
    assert!(walk.ports.contains(&Port::Operation(id)));
    assert!(!walk.ports.contains(&Port::Entry(last)));
    assert!(!walk.ports.contains(&Port::Normal(id)));
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    let (_, Effect::Dispatch(op)) = reports.effects[&id] else {
        panic!()
    };
    assert!(op.normal && op.initialized && !op.result);
    let before = reports.effects.clone();
    checker
        .dispatch_ops
        .get_mut(&id)
        .unwrap()
        .edges
        .push(Edge::new(
            Port::Operation(id),
            Port::Entry(last),
            Route::Next,
        ));
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(reports.effects, before);
}

#[test]
pub(crate) fn dispatch_effects_reject_other_registered_receivers_and_body_prefix_corruption() {
    for fault in 0..10 {
        let (mut checker, reports) = checked("a:1.{};b:2.{->$}", false);
        let mut ops = checker.dispatch_ops.iter();
        let (&id, op) = ops.next().unwrap();
        let block = op.block;
        let other = ops.next().unwrap().1.clone();
        assert!(checker.proofs.receivers.contains(&other.local));
        match fault {
            0 => checker.dispatch_ops.get_mut(&id).unwrap().local = other.local,
            1 => checker.dispatch_ops.get_mut(&id).unwrap().block = other.block,
            2 => checker.dispatch_ops.get_mut(&id).unwrap().input = other.input,
            3 => checker.bodies.get_mut(&block).unwrap().facts.clear(),
            4 => checker.bodies.get_mut(&block).unwrap().links.clear(),
            5 => checker.bodies.get_mut(&block).unwrap().sources.clear(),
            6 => checker.bodies.get_mut(&block).unwrap().storage.clear(),
            7 => {
                checker.bodies.get_mut(&block).unwrap().links[0] =
                    Some(crate::check::dependencies::bodies::Link {
                        parent: 0,
                        role: crate::check::dependencies::bodies::Role::Data,
                    })
            }
            8 => checker.points[id].span = Span::default(),
            9 => checker.bodies.get_mut(&block).unwrap().storage[0] = Some(other.local),
            _ => unreachable!(),
        }
        let before = reports.effects.clone();
        let ops = checker.dispatch_ops.clone();
        assert!(
            checker
                .operation_effects(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(reports.effects, before);
        assert_eq!(checker.dispatch_ops, ops);
    }
}

#[test]
pub(crate) fn dispatch_effects_preserve_receiver_cells_loans_and_caller_specific_errors() {
    let (checker, reports) = checked("n:1;v:(&n).{->*$}", false);
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    let (_, Effect::Dispatch(observed)) = reports.effects[&id] else {
        panic!()
    };
    assert_eq!(observed.local, op.local);
    assert!(
        checker
            .local_reads
            .values()
            .any(|read| read.local == op.local)
    );
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Deref { .. }))
    );
    for (source, code) in [
        ("v:3.{->&$}", "E303"),
        ("n:=1;v:(&n).{n=2;->*$}", "E302"),
        ("v:3.{$=4;->$}", "E305"),
        ("v<{x<int32>;y<int32>}>:{->3.{->x:$}}", "E204"),
        ("v<{x<int32>}>:{->3.{->x:true}}", "E207"),
        ("n:=1;v:(&!n).{}", "B001"),
        ("n:=1;v<{x<int32>}>:{->(&!n).{->x:missing}}", "E201"),
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};v:stop().{missing}",
            "E201",
        ),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
