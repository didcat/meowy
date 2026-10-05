use super::{tests::check, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn output_sources_capture_primary_kinds_and_keep_literal_and_scalar_parts_distinct() {
    for (setup, shape) in [
        (
            "r:3.{->$;->tag:true}",
            Shape::Scalar(ScalarKind::Int {
                bits: 32,
                signed: true,
            }),
        ),
        (
            "n<uint8>:3;r:n.{->$;->tag:true}",
            Shape::Scalar(ScalarKind::Int {
                bits: 8,
                signed: false,
            }),
        ),
        (
            "n<float32>:1.5;r:n.{->$;->tag:true}",
            Shape::Scalar(ScalarKind::Float { bits: 32 }),
        ),
        ("r:true.{->$;->tag:true}", Shape::Scalar(ScalarKind::Bool)),
        (
            "r:\"cat\".{->$;->tag:true}",
            Shape::Scalar(ScalarKind::String),
        ),
        ("r:3.{->tag:true}", Shape::Scalar(ScalarKind::Null)),
        (
            "n<int32><null>:1;r:3.{->n;->tag:true}",
            Shape::Union { members: 2 },
        ),
    ] {
        for method in ["print", "panic"] {
            let source = format!("d:@\"debug\";{setup};d.{method}(\"a{{r}}b{{1}}\")");
            crate::compile(&source).unwrap();
            let (checker, _) = check(&source);
            let op = checker.outputs.values().next().unwrap();
            assert!(op.parts[0].is_none() && op.parts[2].is_none());
            let input = op.parts[1].unwrap();
            assert!(input.primary && input.valid_source(1, op.stopped));
            assert_eq!(input.source, Some(shape));
            assert_eq!(op.parts[3].unwrap().source, None);
            assert!(!op.parts[3].unwrap().primary);
        }
    }
}

#[test]
pub(crate) fn output_sources_keep_never_shapes_checked_suffixes_and_formatting_rejections() {
    let source = "d:@\"debug\";f<null>:(r<{-><never>;tag<boolean>}>){d.print(\"{r}tail{r}{(3.{->$;->tag:true})}\")}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let op = checker.outputs.values().next().unwrap();
    assert_eq!(op.stopped, Some(0));
    assert_ne!(op.owner, 0);
    for part in [0, 2] {
        let input = op.parts[part].unwrap();
        assert_eq!(input.source, Some(Shape::Never));
        assert!(input.valid_source(part, op.stopped));
        assert!(checker.points[input.point].complete);
    }
    assert!(matches!(
        op.parts[3].unwrap().source,
        Some(Shape::Scalar(_))
    ));
    assert!(
        !op.edges
            .iter()
            .any(|edge| matches!(edge.to, Port::Projection { step, .. } if step > 0))
    );
    for setup in [
        "n:1;r:3.{->&n;->tag:true}",
        "r:3.{->[$];->tag:true}",
        "r:3.{->@\"memory\".heap;->tag:true}",
    ] {
        let source = format!("d:@\"debug\";{setup};d.print(r)");
        let mut checker = Checker::new();
        let error = checker
            .block(&crate::parser::parse(&source).unwrap(), None, None)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("formatting"));
        assert!(checker.outputs.is_empty());
    }
}

#[test]
pub(crate) fn output_sources_reject_changed_shapes_before_publication_and_bound_work() {
    let (mut checker, block) = check("d:@\"debug\";r:3.{->$;->tag:true};d.print(r)");
    let hir::Stmt::Expr(value) = block.stmts.last().unwrap() else {
        panic!()
    };
    let hir::ExprKind::Print { parts, .. } = &value.kind else {
        panic!()
    };
    let (&id, op) = checker.outputs.first_key_value().unwrap();
    let op = op.clone();
    let count = checker.output_edges;
    for source in [
        None,
        Some(Shape::Never),
        Some(Shape::Scalar(ScalarKind::Bool)),
        Some(Shape::Scalar(ScalarKind::Int {
            bits: 7,
            signed: true,
        })),
        Some(Shape::List { capacity: 1 }),
        Some(Shape::Union { members: 1 }),
    ] {
        let mut changed = op.parts.clone();
        changed[0].as_mut().unwrap().source = source;
        for replay in [true, false] {
            if !replay {
                checker.outputs.clear();
                checker.output_edges = 0;
            }
            assert!(
                checker
                    .output_operation(id, false, parts, changed.clone(), value.span)
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            if replay {
                assert_eq!(checker.outputs[&id], op);
                assert_eq!(checker.output_edges, count);
            } else {
                assert!(checker.outputs.is_empty());
                assert_eq!(checker.output_edges, 0);
            }
        }
        checker
            .output_operation(id, false, parts, op.parts.clone(), value.span)
            .unwrap();
    }
    let start = checker.flow.work;
    checker
        .output_operation(id, false, parts, op.parts.clone(), value.span)
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.output_operation(id, false, parts, op.parts.clone(), value.span);
        assert_eq!(result.is_ok(), short == 0);
        if short == 1 {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(checker.outputs[&id], op);
        assert_eq!(checker.output_edges, count);
    }
}
