use super::*;

mod boundaries;
mod stages;

pub(super) const SOURCE: &str = "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:3.{->$;->tag:true};out:r.{copy:(($));|v<boolean>|xs<T[5]><U[5]>:[$,1,copy,$,v]}}";

#[test]
pub(crate) fn receiver_list_primaries_keep_sparse_indices_shapes_and_independent_owners() {
    for (init, dispatch) in [("{->3;->tag:true}", false), ("3.{->$;->tag:true}", true)] {
        let source = format!(
            "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){{r:{init};out:r.{{copy:(($));inner:$.{{|v<boolean>|xs<T[5]><U[5]>:[$,1,copy,$,v]}}}}}};v<boolean><string>:true;r:{init};out:r.{{|v<boolean>|xs<T[2]><U[2]>:[$,v]}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 4);
        let mut steps = BTreeMap::new();
        for (&port, &(owner, slot)) in &reports.slot_uses {
            let Port::Projection { point, step } = port else {
                panic!()
            };
            let (_, Effect::List(list)) = &reports.effects[&point] else {
                panic!()
            };
            let input = &list.inputs[step];
            assert!(list.contextual && input.projected);
            assert_eq!(input.plan, Some((true, CoercionKind::Convert)));
            assert_eq!(slot.index, 0);
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(input.source, Some(slots[0].shape));
            assert_eq!(reports.results[&slot.block].1.dispatch.is_some(), dispatch);
            assert_eq!(checker.lists[&point].owner, owner);
            assert_eq!(
                checker
                    .grouped_consumer(&reports, input.point, owner, Span::default())
                    .unwrap(),
                None
            );
            assert_eq!(
                checker
                    .dispatch_consumer(&reports, input.point, owner, Span::default())
                    .unwrap(),
                None
            );
            steps.entry(owner).or_insert_with(Vec::new).push(step);
        }
        assert_eq!(steps[&0], [0]);
        assert_eq!(steps[&1], [0, 2, 3]);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn receiver_list_primaries_keep_scalar_kinds_and_guarded_receiver_scopes() {
    for (ty, setup) in [
        ("uint8", "n<uint8>:7;r:n.{->$;->tag:true}"),
        ("float32", "n<float32>:1.5;r:n.{->$;->tag:true}"),
        ("string", "r:{->\"cat\";->tag:true}"),
        ("null", "r:{->tag:true}"),
        ("boolean", "r:{->true;->tag:true}"),
    ] {
        let select = if ty == "boolean" { "int32" } else { "boolean" };
        let source = format!(
            "<T>:<{ty}><{select}>;<U>:<{ty}><string>;f:(v<{select}><string>){{{setup};out:r.{{|v<{select}>|xs<T[2]><U[2]>:[$,v]}}}}"
        );
        let (_, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 1, "{source}");
    }
    let source = format!("{SOURCE};f(true)");
    let (_, reports) = checked(&source);
    assert_eq!(reports.slot_uses.len(), 3);
}
