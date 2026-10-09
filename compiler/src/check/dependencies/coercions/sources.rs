use super::{
    tests::{expr, prepare},
    *,
};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn coercion_sources_capture_primary_shapes_before_forwarding_or_conversion() {
    let int = hir::Type::Int {
        bits: 32,
        signed: true,
    };
    let union = hir::Type::union(vec![int.clone(), hir::Type::Null]);
    let mut checker = prepare("r:3.{->$;->tag:true}");
    for (target, kind) in [(&int, Kind::Forward), (&union, Kind::Convert)] {
        let (id, value) = checker.expr_point(&expr("r"), Some(target)).unwrap();
        let op = &checker.coercions[&id];
        assert_eq!(
            op.source,
            Some(Shape::Scalar(ScalarKind::Int {
                bits: 32,
                signed: true
            }))
        );
        assert_eq!((op.primary, op.kind), (true, kind));
        assert_eq!(value.ty, *target);
    }
    for (source, shape) in [
        (
            "n<uint8>:3;r:n.{->$;->tag:true};v<uint8><null>:r",
            Shape::Scalar(ScalarKind::Int {
                bits: 8,
                signed: false,
            }),
        ),
        (
            "n<float32>:1.5;r:n.{->$;->tag:true};v<float32>:r",
            Shape::Scalar(ScalarKind::Float { bits: 32 }),
        ),
        (
            "r:true.{->$;->tag:true};v<boolean>:r",
            Shape::Scalar(ScalarKind::Bool),
        ),
        (
            "r:\"cat\".{->$;->tag:true};v<string>:r",
            Shape::Scalar(ScalarKind::String),
        ),
        (
            "r:3.{->tag:true};v<null>:r",
            Shape::Scalar(ScalarKind::Null),
        ),
        (
            "n:1;r:3.{->&n;->tag:true};v<&int32>:r",
            Shape::SharedScalar(ScalarKind::Int {
                bits: 32,
                signed: true,
            }),
        ),
        (
            "r:3.{->[$];->tag:true};v<int32[1]>:r",
            Shape::List { capacity: 1 },
        ),
        (
            "f<boolean>:(r<{-><never>;tag<boolean>}>){->r}",
            Shape::Never,
        ),
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        let projected: Vec<_> = checker.coercions.values().filter(|op| op.primary).collect();
        assert!(!projected.is_empty(), "{source}");
        assert!(
            projected.iter().all(|op| op.source == Some(shape)),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn coercion_sources_keep_nonprimary_and_whole_record_conversion_distinct() {
    for source in [
        "n:1;v<int32><null>:n",
        "r:3.{->$;->tag:true};v<{-><int32>;tag<boolean>}><null>:r",
        "d:@\"debug\";v<boolean>:d.panic(\"stop\")",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap();
        assert!(!checker.coercions.is_empty());
        assert!(
            checker
                .coercions
                .values()
                .all(|op| !op.primary && op.source.is_none())
        );
    }
}

#[test]
pub(crate) fn coercion_sources_reject_malformed_shapes_and_changed_replay_atomically() {
    let int = hir::Type::Int {
        bits: 32,
        signed: true,
    };
    let mut checker = prepare("r:{->3;->tag:true}");
    let (id, _) = checker.expr_point(&expr("r"), Some(&int)).unwrap();
    let op = checker.coercions[&id].clone();
    let count = checker.coercion_edges;
    for source in [
        None,
        Some(Shape::Never),
        Some(Shape::Scalar(ScalarKind::Bool)),
        Some(Shape::Scalar(ScalarKind::Int {
            bits: 7,
            signed: true,
        })),
    ] {
        assert!(
            checker
                .coercion_stages(id, op.input, op.kind, true, source, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(checker.coercions[&id], op);
        assert_eq!(checker.coercion_edges, count);
    }
    for (kind, primary, source, valid) in [
        (Kind::Stopped, true, Some(Shape::Never), true),
        (Kind::Stopped, false, None, true),
        (Kind::Stopped, true, op.source, false),
        (Kind::Forward, false, op.source, false),
        (Kind::Convert, true, None, false),
    ] {
        assert_eq!(kind.valid_source(primary, source), valid);
    }
    let start = checker.flow.work;
    checker
        .coercion_stages(id, op.input, op.kind, true, op.source, op.span)
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.coercion_stages(id, op.input, op.kind, true, op.source, op.span);
        assert_eq!(result.is_ok(), short == 0);
        if short == 1 {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(checker.coercions[&id], op);
        assert_eq!(checker.coercion_edges, count);
    }
}
