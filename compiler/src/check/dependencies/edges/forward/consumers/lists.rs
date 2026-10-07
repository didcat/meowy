use super::{tests::checked, *};
use crate::check::dependencies::CoercionKind;

mod dispatch;
mod limits;
mod receivers;

#[test]
pub(crate) fn list_consumers_link_original_contextual_parts_to_local_record_anchors() {
    let source = "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:{->7;->tag:true};copy:((r));|v<boolean>|xs<T[5]><U[5]>:[r,1,copy,r,v]};f(true)";
    let (mut checker, reports) = checked(source);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let Effect::List(list) = &reports.effects[&id].1 else {
        panic!()
    };
    assert!(list.contextual && list.constructed && list.result);
    let parts = list
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(part, input)| input.projected.then_some(part))
        .collect::<Vec<_>>();
    assert_eq!(parts, [0, 2, 3]);
    assert_eq!(reports.slot_uses.len(), 3);
    for step in parts {
        let input = &list.inputs[step];
        assert_eq!(input.plan, Some((true, CoercionKind::Convert)));
        let port = Port::Projection { point: id, step };
        let (found_owner, slot) = reports.slot_uses[&port];
        assert_eq!((found_owner, slot.index), (owner, 0));
        let anchor = checker
            .grouped_consumer(&reports, input.point, owner, Span::default())
            .unwrap()
            .unwrap();
        assert_eq!(reports.consumers[&anchor], (owner, slot.block));
    }
}

#[test]
pub(crate) fn list_consumers_keep_parameter_sources_and_ordinary_coercion_ports_separate() {
    let source = "<T>:<int32><boolean>;<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<T[1]><string[1]>:[v]}";
    let (_, reports) = checked(source);
    assert!(reports.slot_uses.is_empty());
    let (checker, reports) = checked("<T>:<int32><boolean>;r:{->7;->tag:true};xs<T[2]>:[r,1]");
    assert_eq!(reports.slot_uses.len(), 1);
    let Port::Projection { point, step: 0 } = *reports.slot_uses.first_key_value().unwrap().0
    else {
        panic!()
    };
    assert!(checker.coercions[&point].primary);
    assert!(!checker.lists.contains_key(&point));
}
