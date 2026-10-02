use super::*;
use crate::check::{Checker, dependencies::bodies::tests::check};

#[test]
pub(crate) fn block_layout_keeps_null_primary_field_order_and_shallow_shapes() {
    let (checker, block) = check("->z:=1;->a:[1];->b:{->tag:true}");
    let layout = &checker.bodies[&block.id].layout;
    let Layout::Slots(slots) = layout else {
        panic!("record layout");
    };
    assert_eq!(slots[0].field, None);
    assert_eq!(
        slots[0].shape,
        Shape::Scalar(super::super::super::ScalarKind::Null)
    );
    assert!(!slots[0].mutable);
    let hir::Type::Record { fields, .. } = &block.ty else {
        panic!("record type");
    };
    for (slot, field) in slots[1..].iter().zip(fields) {
        assert_eq!(slot.field.as_ref(), Some(&field.name));
        assert_eq!(slot.mutable, field.mutable);
        assert_eq!(slot.shape, Completion::of(&field.ty).result);
    }
    assert!(slots.iter().any(|slot| slot.mutable));
    assert!(
        slots
            .iter()
            .any(|slot| slot.shape == Shape::List { capacity: 1 })
    );
    assert!(
        slots
            .iter()
            .any(|slot| slot.shape == Shape::Record { fields: 1 })
    );
    assert!(layout.matches(&block.ty));
    assert_eq!(checker.body_slots, 6);
    assert_eq!(checker.body_names, 6);
}

#[test]
pub(crate) fn block_layout_distinguishes_roots_stops_and_unknown_unions() {
    for source in ["", "->1", "->[1]", "n:1;->&n"] {
        let (checker, block) = check(source);
        let layout = &checker.bodies[&block.id].layout;
        assert!(matches!(layout, Layout::Slots(slots) if slots.len() == 1));
        assert!(layout.matches(&block.ty));
    }
    for (source, expected) in [
        ("flag:=false;|flag|->1", Layout::Unknown),
        ("'loop{'loop.restart()}", Layout::Stopped),
    ] {
        let (checker, block) = check(source);
        assert_eq!(checker.bodies[&block.id].layout, expected);
        assert!(expected.matches(&block.ty));
    }
    let ty = hir::Type::Record {
        primary: Box::new(hir::Type::Union(vec![hir::Type::Bool, hir::Type::Null])),
        fields: vec![],
    };
    let (layout, count, names) =
        Layout::capture(&ty, &mut Flow::new(), 0, 0, Span::default()).unwrap();
    assert_eq!((count, names), (1, 0));
    assert!(
        matches!(layout, Layout::Slots(slots) if slots[0].shape == Shape::Union { members: 2 })
    );
    let ty = hir::Type::Foundation(hir::FoundationType::Allocator);
    let (layout, count, names) =
        Layout::capture(&ty, &mut Flow::new(), 0, 0, Span::default()).unwrap();
    assert_eq!((layout, count, names), (Layout::Unknown, 0, 0));
}

#[test]
pub(crate) fn block_layout_replay_and_endpoints_reject_changed_slot_identity_atomically() {
    for case in 0..4 {
        let (mut checker, mut block) = check("->a:1;->z:true");
        let span = checker.bodies[&block.id].span;
        let prior = format!("{:?}", checker.bodies);
        let counts = (checker.body_facts, checker.body_slots, checker.body_names);
        let edges = checker.endpoints.clone();
        checker.track_body(&block, span).unwrap();
        checker.block_endpoints(&block, span).unwrap();
        let hir::Type::Record { fields, .. } = &mut block.ty else {
            panic!("record type");
        };
        match case {
            0 => fields[0].name = "b".into(),
            1 => fields[0].mutable = true,
            2 => fields.swap(0, 1),
            3 => fields[0].ty = hir::Type::Bool,
            _ => unreachable!(),
        }
        assert!(checker.track_body(&block, span).is_err());
        assert!(checker.block_endpoints(&block, span).is_err());
        assert_eq!(format!("{:?}", checker.bodies), prior);
        assert_eq!(
            (checker.body_facts, checker.body_slots, checker.body_names),
            counts
        );
        assert_eq!(checker.endpoints, edges);
    }
}

#[test]
pub(crate) fn block_layout_caps_copies_and_keeps_failed_publication_atomic() {
    let span = Span::default();
    for (count, names, succeeds) in [
        (MAX_SLOTS - 1, 0, true),
        (MAX_SLOTS, 0, false),
        (1, MAX_NAMES, true),
        (1, MAX_NAMES + 1, false),
    ] {
        let ty = hir::Type::Record {
            primary: Box::new(hir::Type::Null),
            fields: vec![
                hir::Field {
                    name: "x".repeat(names),
                    ty: hir::Type::Bool,
                    mutable: false
                };
                count
            ],
        };
        assert_eq!(
            Layout::capture(&ty, &mut Flow::new(), 0, 0, span).is_ok(),
            succeeds
        );
    }
    let block = hir::Block {
        id: 0,
        ty: hir::Type::Record {
            primary: Box::new(hir::Type::Null),
            fields: vec![hir::Field {
                name: "n".into(),
                ty: hir::Type::Bool,
                mutable: false,
            }],
        },
        stmts: vec![],
    };
    for case in 0..4 {
        let mut checker = Checker::new();
        checker.body_slots = MAX_BODY_SLOTS - 2;
        checker.body_names = MAX_BODY_NAMES - 1;
        match case {
            0 => {}
            1 => checker.body_slots += 1,
            2 => checker.body_names += 1,
            3 => checker.flow.work = crate::flow::MAX_PROOF_WORK - 4,
            _ => unreachable!(),
        }
        let counts = (checker.body_slots, checker.body_names);
        assert_eq!(checker.track_body(&block, span).is_ok(), case == 0);
        if case == 0 {
            assert_eq!(
                (checker.body_slots, checker.body_names),
                (MAX_BODY_SLOTS, MAX_BODY_NAMES)
            );
            checker.track_body(&block, span).unwrap();
            assert_eq!(
                (checker.body_slots, checker.body_names),
                (MAX_BODY_SLOTS, MAX_BODY_NAMES)
            );
        } else {
            assert!(checker.bodies.is_empty());
            assert_eq!((checker.body_slots, checker.body_names), counts);
        }
    }
    let (_, block) = check("->n:1");
    let mut checker = Checker::new();
    checker.body_facts = MAX_BODY_FACTS;
    assert!(checker.track_body(&block, span).is_err());
    assert!(checker.bodies.is_empty());
    assert_eq!((checker.body_slots, checker.body_names), (0, 0));
}
