use super::{tests::check, *};

#[test]
pub(crate) fn dispatch_completion_keeps_receiver_and_body_stops_independent() {
    for composed in [false, true] {
        for (tail, input_normal, normal) in [
            ("3.{}", true, true),
            ("3.{->x:$}", true, true),
            ("stop().{}", false, false),
            ("3.{stop()}", true, false),
        ] {
            let prefix = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};";
            let source = if composed {
                let slots = if tail == "3.{->x:$}" {
                    "->y:4"
                } else {
                    "->x:4;->y:4"
                };
                format!("{prefix}f<{{x<int32>;y<int32>}}>:(){{->{tail};{slots}}}")
            } else {
                format!("{prefix}v:{tail}")
            };
            crate::compile(&source).unwrap();
            let (checker, _) = check(&source);
            let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
            assert_eq!(
                (op.input_normal, op.normal),
                (input_normal, normal),
                "{source}"
            );
            assert_eq!(
                op.edges.iter().any(|edge| edge.to == Port::Operation(id)),
                input_normal
            );
            assert_eq!(
                op.edges.iter().any(|edge| edge.to == Port::Normal(id)),
                input_normal && normal
            );
            assert_eq!(
                checker.sequences[&SequenceSource::Block(op.block)].items[0],
                None
            );
        }
    }
}

#[test]
pub(crate) fn dispatch_completion_preserves_partial_composition_and_replay_identity() {
    let source = "v<{x<int32>;y<int32>}>:{->((3.{->x:$}));->y:4}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let op = checker.dispatch_ops.values().next().unwrap();
    assert!(op.input_normal && op.normal);
    let (mut checker, body) = check("v:3.{->$}");
    let hir::Stmt::Bind { value, .. } = &body.stmts[0] else {
        panic!()
    };
    let hir::ExprKind::Block(body) = &value.kind else {
        panic!()
    };
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    let op = op.clone();
    let count = checker.dispatch_edges;
    checker
        .dispatch_operation(id, op.input, op.local, body, op.span)
        .unwrap();
    assert_eq!(checker.dispatch_ops[&id], op);
    for fault in 0..3 {
        let prior = checker.dispatch_ops.get_mut(&id).unwrap();
        if fault == 0 {
            prior.input_normal = false;
        } else if fault == 1 {
            prior.normal = false;
        } else {
            prior.receiver = Shape::Other;
        }
        let before = checker.dispatch_ops[&id].clone();
        assert!(
            checker
                .dispatch_operation(id, op.input, op.local, body, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(checker.dispatch_ops[&id], before);
        assert_eq!(checker.dispatch_edges, count);
        checker.dispatch_ops.insert(id, op.clone());
    }
}

#[test]
pub(crate) fn dispatch_receiver_shape_is_shallow_and_independent_of_body_results() {
    use crate::check::dependencies::ScalarKind;
    for (source, shape, normal) in [
        (
            "v:3.{->false}",
            Shape::Scalar(ScalarKind::Int {
                bits: 32,
                signed: true,
            }),
            true,
        ),
        ("v:true.{->3}", Shape::Scalar(ScalarKind::Bool), true),
        ("v:\"x\".{->3}", Shape::Scalar(ScalarKind::String), true),
        ("v:null.{->3}", Shape::Scalar(ScalarKind::Null), true),
        (
            "n:1;v:(&n).{->3}",
            Shape::Reference(hir::ReferenceMode::Shared),
            true,
        ),
        ("v:{->n:1}.{->3}", Shape::Record { fields: 1 }, true),
        ("v:[1,2].{->3}", Shape::List { capacity: 2 }, true),
        (
            "n<int32><null>:1;v:n.{->3}",
            Shape::Union { members: 2 },
            true,
        ),
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};v:stop().{}",
            Shape::Never,
            false,
        ),
        (
            "d:@\"debug\";v:3.{d.panic(\"stop\")}",
            Shape::Scalar(ScalarKind::Int {
                bits: 32,
                signed: true,
            }),
            false,
        ),
    ] {
        crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        let (checker, _) = check(source);
        let op = checker.dispatch_ops.values().next().unwrap();
        assert_eq!(op.receiver, shape, "{source}");
        assert_eq!(op.input_normal, shape != Shape::Never);
        assert_eq!(op.normal, normal);
    }
}
