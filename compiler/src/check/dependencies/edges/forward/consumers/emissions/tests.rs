use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::Sources;

#[test]
pub(crate) fn emission_consumers_link_checked_slots_through_copies_and_independent_owners() {
    let source = "f<{z<int32>;a<boolean>}>:(){r:{->z:1;->a:true};s:((r));->((s))};r:{->7;->z:2;->a:false};s:((r));v:'out{{'out->((s))}}";
    let (mut checker, reports) = checked(source);
    let mut count = 0;
    for id in checker.emissions.keys().copied().collect::<Vec<_>>() {
        let op = &checker.emissions[&id];
        if op.composed.is_none() {
            continue;
        }
        let (input, owner) = (op.input, op.owner);
        let anchor = checker
            .grouped_consumer(&reports, input, owner, Span::default())
            .unwrap()
            .unwrap();
        let (_, block) = reports.consumers[&anchor];
        let op = &checker.emissions[&id];
        let Layout::Slots(slots) = &checker.bodies[&block].layout else {
            panic!()
        };
        for (index, target) in op.targets.iter().enumerate() {
            assert_eq!(
                reports.slot_uses[&Port::Emission(target.id)],
                (op.owner, Slot { block, index })
            );
            assert_eq!(slots[index].field, target.field);
            count += 1;
        }
        assert!(matches!(&reports.effects[&id].1, Effect::Emission(op) if op.result));
    }
    assert_eq!(count, 6);
    assert_eq!(reports.slot_uses.len(), count);
    assert!(reports.slot_uses.values().any(|(owner, _)| *owner == 1));
    let before = format!("{:?}", reports.results);
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );
    assert_eq!(format!("{:?}", reports.results), before);
}

#[test]
pub(crate) fn emission_consumers_preserve_unknown_values_and_partial_destination_layouts() {
    for source in [
        "v:{->{->n:=1}}",
        "v:{->{->inner:{->n:1}}}",
        "v<{x<int32>;y<int32>}>:{->{->x:1};->y:2}",
        "d:@\"debug\";flag:=false;row:'out{|flag|{'out->{->gone:9};d.panic(\"stop\")};->x:3}",
    ] {
        let (checker, reports) = checked(source);
        let op = checker
            .emissions
            .values()
            .find(|op| op.composed.is_some())
            .unwrap();
        assert_eq!(op.targets.len(), 2);
        let (owner, slot) = reports.slot_uses[&Port::Emission(op.targets[1].id)];
        assert_eq!(owner, op.owner);
        assert_eq!(slot.index, 1);
        if source.contains("n:=") || source.contains("inner:") {
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[1],
                Sources::Unknown
            );
        }
        assert_ne!(slot.block, op.targets[1].block);
    }
}

#[test]
pub(crate) fn emission_consumers_retain_initialized_targets_without_destination_completion() {
    let source = "d:@\"debug\";v:{->{->n:1};d.panic(\"stop\")}";
    let (checker, reports) = checked(source);
    let op = checker
        .emissions
        .values()
        .find(|op| op.composed.is_some())
        .unwrap();
    assert!(
        reports
            .blocks
            .get(&op.targets[0].block)
            .is_none_or(|(_, block)| !block.result)
    );
    for target in &op.targets {
        assert!(reports.slot_uses.contains_key(&Port::Emission(target.id)));
    }
}

#[test]
pub(crate) fn emission_consumers_keep_direct_values_and_opaque_sources_separate() {
    for source in [
        "v:{->1;->n:2}",
        "r:{};v:{->r}",
        "r:{->n:1};v:{->named:r}",
        "f<{n<int32>}>:(){->n:1};v:{->f()}",
        "f:(r<{n<int32>}>){v:{->r}}",
        "r:{->n:1};p:&r;v:{->*p}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn emission_consumers_link_empty_records_to_their_primary_slot() {
    let (checker, reports) = checked("r<{}>:{};v:{->r}");
    let op = checker
        .emissions
        .values()
        .find(|op| op.composed.is_some())
        .unwrap();
    assert_eq!(op.targets.len(), 1);
    assert_eq!(reports.slot_uses.len(), 1);
    let (_, slot) = reports.slot_uses[&Port::Emission(op.targets[0].id)];
    assert_eq!(slot.index, 0);
    assert_eq!(
        checker.bodies[&slot.block].completion.result,
        Shape::Record { fields: 0 }
    );
}
