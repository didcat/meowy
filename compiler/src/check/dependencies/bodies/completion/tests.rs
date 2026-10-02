use super::*;
use crate::{
    ast::Span,
    check::{
        Checker,
        dependencies::bodies::{Body, tests::check},
    },
};

#[test]
pub(crate) fn block_completion_keeps_checked_shapes_and_partial_results() {
    for (source, result) in [
        ("", Shape::Scalar(ScalarKind::Null)),
        (
            "->1",
            Shape::Scalar(ScalarKind::Int {
                bits: 32,
                signed: true,
            }),
        ),
        ("->n:1;->tag:true", Shape::Record { fields: 2 }),
        ("flag:=false;|flag|->1", Shape::Union { members: 2 }),
        ("-> [1,2]", Shape::List { capacity: 2 }),
        ("n:1;->&n", Shape::Reference(hir::ReferenceMode::Shared)),
        ("'loop{'loop.restart()}", Shape::Never),
    ] {
        let (checker, block) = check(source);
        let body = &checker.bodies[&block.id];
        assert_eq!(body.span, Span::new(0, source.len()));
        assert_eq!(body.parent, None);
        assert_eq!(body.completion.result, result);
        assert_eq!(body.completion.normal, result != Shape::Never);
        assert!(body.completion.valid());
    }
}

#[test]
pub(crate) fn block_completion_retains_function_owners_and_nested_source_spans() {
    let source = "f<int32>:(){row:{->7;->tag:true};->row+0};{->false}";
    let (checker, root) = check(source);
    let function = checker.functions[0].as_ref().unwrap();
    let body = &checker.bodies[&function.body.id];
    assert_ne!(body.owner, checker.bodies[&root.id].owner);
    assert_eq!(body.parent, None);
    assert_eq!(
        body.completion.result,
        Shape::Scalar(ScalarKind::Int {
            bits: 32,
            signed: true
        })
    );
    assert_eq!(
        &source[body.span.start..body.span.end],
        "(){row:{->7;->tag:true};->row+0}"
    );
    let nested = checker
        .bodies
        .values()
        .find(|body| body.completion.result == Shape::Record { fields: 1 })
        .unwrap();
    assert_eq!(nested.owner, body.owner);
    assert!(nested.parent.is_some());
    assert_eq!(
        &source[nested.span.start..nested.span.end],
        "{->7;->tag:true}"
    );
}

#[test]
pub(crate) fn block_completion_bounds_shallow_counts_without_copying_types() {
    for count in [0, 1, 2, MAX_BODY_RESULTS, MAX_BODY_RESULTS + 1] {
        let fields = vec![
            hir::Field {
                name: String::new(),
                ty: hir::Type::Never,
                mutable: false
            };
            count
        ];
        let record = hir::Type::Record {
            primary: Box::new(hir::Type::Never),
            fields,
        };
        let list = hir::Type::List {
            element: Box::new(record.clone()),
            capacity: count,
        };
        let union = hir::Type::Union(vec![hir::Type::Null; count]);
        for (ty, expected) in [
            (
                &record,
                if count <= MAX_BODY_RESULTS {
                    Shape::Record { fields: count }
                } else {
                    Shape::Other
                },
            ),
            (
                &list,
                if count <= crate::list::MAX_CAPACITY {
                    Shape::List { capacity: count }
                } else {
                    Shape::Other
                },
            ),
            (
                &union,
                if (2..=MAX_BODY_RESULTS).contains(&count) {
                    Shape::Union { members: count }
                } else {
                    Shape::Other
                },
            ),
        ] {
            let completion = Completion::of(ty);
            assert_eq!(completion.result, expected);
            assert!(completion.normal && completion.valid());
        }
    }
    for result in [
        Shape::Record {
            fields: MAX_BODY_RESULTS + 1,
        },
        Shape::List {
            capacity: crate::list::MAX_CAPACITY + 1,
        },
        Shape::Union { members: 1 },
        Shape::Union {
            members: MAX_BODY_RESULTS + 1,
        },
        Shape::Scalar(ScalarKind::Int {
            bits: 7,
            signed: true,
        }),
        Shape::Scalar(ScalarKind::Float { bits: 16 }),
        Shape::Never,
    ] {
        assert!(
            !Completion {
                normal: true,
                result
            }
            .valid()
        );
    }
}

#[test]
pub(crate) fn block_completion_replay_and_endpoint_agreement_are_atomic() {
    for case in 0..4 {
        let (mut checker, mut block) = check("->1");
        let prior = format!("{:?}", checker.bodies);
        let count = checker.body_facts;
        let edges = checker.endpoints.clone();
        let span = checker.bodies[&block.id].span;
        checker.track_body(&block, span).unwrap();
        let mut replay_span = span;
        match case {
            0 => block.ty = hir::Type::Bool,
            1 => block.ty = hir::Type::Never,
            2 => replay_span.end += 1,
            3 => checker.owner += 1,
            _ => unreachable!(),
        }
        assert!(checker.track_body(&block, replay_span).is_err());
        assert!(checker.block_endpoints(&block, replay_span).is_err());
        assert_eq!(format!("{:?}", checker.bodies), prior);
        assert_eq!(checker.body_facts, count);
        assert_eq!(checker.endpoints, edges);
    }
}

#[test]
pub(crate) fn block_completion_bounds_empty_body_maps_and_work_atomically() {
    let span = Span::default();
    let mut checker = Checker::new();
    for id in 0..MAX_BODY_RESULTS {
        checker.bodies.insert(
            id,
            Body {
                owner: 0,
                parent: None,
                span,
                completion: Completion::of(&hir::Type::Null),
                layout: super::super::Layout::Slots(vec![super::super::layout::Slot {
                    field: None,
                    mutable: false,
                    shape: Shape::Scalar(ScalarKind::Null),
                }]),
                facts: Vec::new(),
                links: Vec::new(),
                storage: Vec::new(),
                sources: Vec::new(),
            },
        );
    }
    let mut block = hir::Block {
        id: MAX_BODY_RESULTS,
        ty: hir::Type::Null,
        stmts: Vec::new(),
    };
    assert!(
        checker
            .track_body(&block, span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.bodies.len(), MAX_BODY_RESULTS);
    block.id = 0;
    checker.track_body(&block, span).unwrap();
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.track_body(&block, span).is_err());
    assert_eq!(checker.bodies.len(), MAX_BODY_RESULTS);
    assert_eq!(checker.body_facts, 0);
}
