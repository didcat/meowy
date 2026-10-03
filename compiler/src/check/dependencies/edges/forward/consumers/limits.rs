use super::{tests::checked, *};
use crate::check::dependencies::{BinaryClass, ScalarKind, UnaryKind};

pub(super) fn rejected(checker: &mut Checker, reports: &Reports) {
    let effects = reports.effects.clone();
    let blocks = reports.blocks.clone();
    let results = reports.results.clone();
    let consumers = reports.consumers.clone();
    let uses = reports.slot_uses.clone();
    let parts = reports.parts;
    let counts = checker.edge_counts();
    let error = checker.slot_uses(reports, Span::default()).unwrap_err();
    assert!(error.message.contains("identity"), "{error:?}");
    assert_eq!(reports.effects, effects);
    assert_eq!(reports.blocks, blocks);
    assert_eq!(reports.results, results);
    assert_eq!(reports.consumers, consumers);
    assert_eq!(reports.slot_uses, uses);
    assert_eq!(reports.parts, parts);
    assert_eq!(checker.edge_counts(), counts);
}

#[test]
pub(crate) fn slot_uses_share_exact_map_and_work_limits_without_payload() {
    let (mut checker, mut reports) =
        checked("a:{->n:1}.n;b:-{->2;->tag:true};c:{->3;->tag:false}+1");
    let uses = reports.slot_uses.clone();
    let effects = reports.effects.clone();
    let results = reports.results.clone();
    let base = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len();
    let limit = base + uses.len();
    assert_eq!(uses.len(), 3);
    reports.parts = 0;
    let before = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        uses
    );
    let work = checker.flow.work - before;
    for room in [limit - 1, base - 1] {
        assert!(
            checker
                .slot_uses_limited(&reports, Span::default(), room)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let actual = checker.slot_uses_limited(&reports, Span::default(), limit);
        assert_eq!(actual.is_ok(), short == 0);
        if let Ok(actual) = actual {
            assert_eq!(actual, uses);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        }
    }
    assert_eq!(reports.slot_uses, uses);
    assert_eq!(reports.effects, effects);
    assert_eq!(reports.results, results);
    assert_eq!(reports.parts, 0);
}

#[test]
pub(crate) fn slot_uses_keep_duplicate_visits_flags_and_projection_steps() {
    let (mut checker, mut reports) =
        checked("a:{->n:1}.n;b:-{->2;->tag:true};c:{->3;->tag:false}+1");
    let expected = reports.slot_uses.clone();
    let effects = reports.effects.clone();
    let results = reports.results.clone();
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert_eq!(reports.effects, effects);
    reports.parts = 0;
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    for (&port, (_, slot)) in &expected {
        match port {
            Port::Operation(id) => assert!(matches!(effects[&id].1, Effect::Field { .. })),
            Port::Projection { point, step } => {
                assert_eq!(slot.index, 0);
                match &effects[&point].1 {
                    Effect::Unary(op) => assert!(step == 0 && op.projected),
                    Effect::Binary(op) => assert!(op.projected[step]),
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }
    assert_eq!(reports.results, results);
    assert_eq!(reports.slot_uses, expected);
    assert_eq!(reports.parts, 0);
}

#[test]
pub(crate) fn slot_uses_reject_primary_headers_and_impossible_stages_atomically() {
    for unary in [true, false] {
        for fault in 0..12 {
            let source = if unary {
                "a:{->n:1}.n;b:-{->2;->tag:true}"
            } else {
                "a:{->n:1}.n;b:{->2;->tag:true}+1"
            };
            let (mut checker, mut reports) = checked(source);
            let id = reports
                .effects
                .iter()
                .find_map(|(&id, (_, effect))| {
                    matches!(effect, Effect::Unary(_) | Effect::Binary(_)).then_some(id)
                })
                .unwrap();
            assert!(checker.fields.keys().all(|field| *field < id));
            if fault == 10 {
                reports.effects.get_mut(&id).unwrap().0 += 1;
            } else if fault == 11 {
                reports.index.operations.remove(&id);
            } else {
                match &mut reports.effects.get_mut(&id).unwrap().1 {
                    Effect::Unary(op) => match fault {
                        0 => op.input = usize::MAX,
                        1 => op.op = UnaryKind::Not,
                        2 => op.ty = ScalarKind::Bool,
                        3 => op.primary = false,
                        4 => op.checked = false,
                        5 => op.control = !op.control,
                        6 => {
                            op.projected = false;
                            op.operation = false;
                            op.result = false;
                        }
                        7 => checker.points[id].complete = false,
                        8 => checker.unaries.get_mut(&id).unwrap().primary = false,
                        9 => {
                            checker.unaries.get_mut(&id).unwrap().edges.pop().unwrap();
                        }
                        _ => unreachable!(),
                    },
                    Effect::Binary(op) => match fault {
                        0 => op.inputs.swap(0, 1),
                        1 => op.op = "-",
                        2 => op.types.result = BinaryClass::Scalar(ScalarKind::Bool),
                        3 => op.plan.primary[0] = false,
                        4 => op.plan.checked = false,
                        5 => op.control = !op.control,
                        6 => {
                            op.projected = [false; 2];
                            op.operation = false;
                            op.result = false;
                        }
                        7 => op.projected[1] = true,
                        8 => op.plan.normal[0] = false,
                        9 => op.plan.equality = true,
                        _ => unreachable!(),
                    },
                    _ => panic!(),
                }
            }
            reports.parts = 0;
            rejected(&mut checker, &reports);
        }
    }
    for result in [false, true] {
        let (mut checker, mut reports) =
            checked("d:@\"debug\";a:{->n:1}.n;b:{->2;->tag:true}+{d.panic(\"stop\")}");
        let op = reports
            .effects
            .values_mut()
            .find_map(|(_, effect)| {
                if let Effect::Binary(op) = effect {
                    Some(op)
                } else {
                    None
                }
            })
            .unwrap();
        if result {
            op.result = true;
        } else {
            op.operation = true;
        }
        rejected(&mut checker, &reports);
    }
}

#[test]
pub(crate) fn slot_uses_discard_late_field_and_primary_source_failures() {
    for primary in [false, true] {
        for fault in 0..4 {
            let source = if primary {
                "a:{->n:1}.n;b:-{->2;->tag:true}"
            } else {
                "a:{->n:1}.n;b:{->n:2}.n"
            };
            let (mut checker, mut reports) = checked(source);
            let (_, slot) = reports
                .slot_uses
                .values()
                .max_by_key(|(_, slot)| slot.block)
                .unwrap();
            let block = slot.block;
            let input = reports.results[&block].1.consumer.unwrap();
            match fault {
                0 => reports.consumers.get_mut(&input).unwrap().0 += 1,
                1 => reports.results.get_mut(&block).unwrap().1.consumer = None,
                2 => reports.blocks.get_mut(&block).unwrap().1.result = false,
                3 => reports.results.get_mut(&block).unwrap().1.slots = None,
                _ => unreachable!(),
            }
            reports.parts = 0;
            rejected(&mut checker, &reports);
        }
    }
}
