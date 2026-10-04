use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn field_slot_links_reject_headers_sources_and_selected_shape_atomically() {
    for fault in 0..18 {
        let (mut checker, mut reports) = checked("a:{->n:1}.n;b:{->n:2}.n");
        let (&id, field) = checker.fields.last_key_value().unwrap();
        let input = field.input;
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let other = reports.slot_uses.first_key_value().unwrap().1.1.block;
        match fault {
            0..=4 => {
                let Effect::Field {
                    input,
                    index,
                    load,
                    normal,
                    control,
                    ..
                } = &mut reports.effects.get_mut(&id).unwrap().1
                else {
                    panic!()
                };
                match fault {
                    0 => *input = usize::MAX,
                    1 => *index += 1,
                    2 => *load = true,
                    3 => *normal = false,
                    4 => *control = !*control,
                    _ => unreachable!(),
                }
            }
            5 => checker.fields.get_mut(&id).unwrap().count += 1,
            6 => reports.consumers.get_mut(&input).unwrap().0 += 1,
            7 => reports.consumers.get_mut(&input).unwrap().1 = other,
            8 => {
                reports.results.remove(&slot.block);
            }
            9 => {
                reports.blocks.remove(&slot.block);
            }
            10 => reports.results.get_mut(&slot.block).unwrap().1.consumer = None,
            11 => reports.results.get_mut(&slot.block).unwrap().0 += 1,
            12 => reports.blocks.get_mut(&slot.block).unwrap().1.span.end += 1,
            13 => reports.blocks.get_mut(&slot.block).unwrap().1.result = false,
            14 => reports
                .results
                .get_mut(&slot.block)
                .unwrap()
                .1
                .slots
                .as_mut()
                .unwrap()
                .clear(),
            15 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                slots[slot.index].shape = Shape::Never;
            }
            16 => {
                let field = checker.fields.get_mut(&id).unwrap();
                field.normal = false;
                field.edges.pop();
                let Effect::Field { normal, .. } = &mut reports.effects.get_mut(&id).unwrap().1
                else {
                    panic!()
                };
                *normal = false;
            }
            17 => reports.results.get_mut(&slot.block).unwrap().1.slots = None,
            _ => unreachable!(),
        }
        let effects = reports.effects.clone();
        let blocks = reports.blocks.clone();
        let results = reports.results.clone();
        let consumers = reports.consumers.clone();
        let uses = reports.slot_uses.clone();
        let parts = reports.parts;
        let error = checker.slot_uses(&reports, Span::default()).unwrap_err();
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(reports.effects, effects);
        assert_eq!(reports.blocks, blocks);
        assert_eq!(reports.results, results);
        assert_eq!(reports.consumers, consumers);
        assert_eq!(reports.slot_uses, uses);
        assert_eq!(reports.parts, parts);
    }
}

#[test]
pub(crate) fn field_slot_links_distinguish_nested_same_names_and_unknown_values() {
    let (checker, reports) = checked("x:{->n:=1}.n;y:{->n:{->n:2}.n}.n;z:{->n:{->n:3}}.n");
    assert_eq!(reports.slot_uses.len(), 4);
    let mut blocks = std::collections::BTreeSet::new();
    let mut unknown = 0;
    for (&port, (_, slot)) in &reports.slot_uses {
        let Port::Operation(id) = port else { panic!() };
        assert!(blocks.insert(slot.block));
        let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
            panic!()
        };
        assert_eq!(slots[slot.index].field.as_deref(), Some("n"));
        let result = &reports.results[&slot.block].1;
        assert_eq!(result.consumer, Some(checker.fields[&id].input));
        unknown += usize::from(result.slots.as_ref().unwrap()[slot.index] == Sources::Unknown);
    }
    assert_eq!(unknown, 2);
}

#[test]
pub(crate) fn field_slot_links_follow_operation_visits_without_normal_inference() {
    let (mut checker, mut reports) = checked("value:{->n:1}.n");
    let (&id, field) = checker.fields.first_key_value().unwrap();
    let owner = field.owner;
    let expected = reports.slot_uses.clone();
    let block = expected[&Port::Operation(id)].1.block;
    reports.blocks.get_mut(&block).unwrap().1.normal = false;
    reports.entries.get_mut(&owner).unwrap().1.ports = vec![Port::Operation(id)];
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert!(matches!(
        reports.effects[&id].1,
        Effect::Field {
            operation: true,
            result: false,
            ..
        }
    ));
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    reports.entries.get_mut(&owner).unwrap().1.ports = vec![Port::Normal(id)];
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert!(matches!(
        reports.effects[&id].1,
        Effect::Field {
            operation: false,
            result: true,
            ..
        }
    ));
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
pub(crate) fn field_slot_links_accept_seeded_never_slots_without_result_claims() {
    let (mut checker, mut reports) = checked("value:{->n:1}.n");
    let (&id, field) = checker.fields.first_key_value().unwrap();
    let owner = field.owner;
    let expected = reports.slot_uses.clone();
    let slot = expected[&Port::Operation(id)].1;
    let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout else {
        panic!()
    };
    slots[slot.index].shape = Shape::Never;
    reports
        .results
        .get_mut(&slot.block)
        .unwrap()
        .1
        .slots
        .as_mut()
        .unwrap()[slot.index] = Sources::Unknown;
    let field = checker.fields.get_mut(&id).unwrap();
    field.normal = false;
    field.edges.pop();
    checker.field_edges -= 1;
    reports.index = checker.forward_index(Span::default()).unwrap();
    reports.entries.get_mut(&owner).unwrap().1.ports = vec![Port::Operation(id)];
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    assert!(matches!(
        reports.effects[&id].1,
        Effect::Field { normal: false, .. }
    ));
    assert!(!reports.entries[&owner].1.ports.contains(&Port::Normal(id)));
}
