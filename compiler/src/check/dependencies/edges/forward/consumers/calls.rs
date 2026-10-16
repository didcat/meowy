use super::{tests::checked, *};
use crate::check::dependencies::{CoercionKind, edges::forward::results::Sources};
use std::collections::BTreeSet;

mod stages;

pub(super) fn projection(checker: &Checker, arg: PointId) -> PointId {
    let mut id = arg;
    for _ in 0..16 {
        let child = if let Some(op) = checker.coercions.get(&id) {
            if op.primary {
                return id;
            }
            assert_eq!(op.kind, CoercionKind::Forward);
            op.input
        } else if let Some(group) = checker.group_inputs.get(&id) {
            group.input
        } else if let Some(edges) = checker.region_edges.get(&id) {
            let Port::Entry(child) = edges[0].to else {
                panic!()
            };
            child
        } else {
            panic!("argument {arg} has no projection path at {id}")
        };
        assert_eq!(checker.points[child].parent, Some(id));
        id = child;
    }
    panic!("argument {arg} projection path is too deep")
}

#[test]
pub(crate) fn call_argument_primaries_reuse_coercion_ports_and_caller_sources() {
    for target in ["int32", "int32><null"] {
        for input in [
            "({->7;->tag:true}~<{-><int32>;tag<boolean>}>)",
            "r",
            "((r))",
            "r~<{-><int32>;tag<boolean>}>",
        ] {
            let source = format!(
                "f:(a<{target}>){{}};r:{{->7;->tag:true}};alias:f;f({input});({input}).(alias);outer:(n<int32>){{r:{{->n;->tag:true}};f({input})}}"
            );
            let (mut checker, reports) = checked(&source);
            assert_eq!(checker.invocations.len(), 3);
            assert_eq!(reports.slot_uses.len(), 3);
            let mut owners = BTreeSet::new();
            for call in checker.invocations.values() {
                let [arg] = call.args[..] else { panic!() };
                let id = projection(&checker, arg);
                let op = &checker.coercions[&id];
                let (_, Effect::Coercion(observed)) = &reports.effects[&id] else {
                    panic!()
                };
                assert_eq!(checker.points[arg].parent, Some(call.point));
                assert_eq!(checker.points[op.input].parent, Some(id));
                assert_eq!(op.owner, call.owner);
                assert!(op.primary && observed.projected && observed.result);
                assert_eq!(op.kind == CoercionKind::Convert, target.ends_with("<null"));
                assert_eq!(observed.operation, op.kind == CoercionKind::Convert);
                let port = Port::Projection { point: id, step: 0 };
                let (owner, slot) = reports.slot_uses[&port];
                assert_eq!((owner, slot.index), (call.owner, 0));
                assert_ne!(owner, call.function + 1);
                assert_eq!(checker.bodies[&slot.block].owner, owner);
                let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(op.source, Some(layout[0].shape));
                let ports = &reports.entries[&owner].1.ports;
                let projection = ports.iter().position(|seen| *seen == port).unwrap();
                let argument = ports
                    .iter()
                    .position(|seen| *seen == Port::Normal(arg))
                    .unwrap();
                let operation = ports
                    .iter()
                    .position(|seen| *seen == Port::Operation(call.point))
                    .unwrap();
                assert!(projection < argument && argument < operation);
                assert!(!reports.slot_uses.contains_key(&Port::Projection {
                    point: call.point,
                    step: 0
                }));
                owners.insert(owner);
            }
            assert_eq!(owners.len(), 2);
            assert!(owners.contains(&0));
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
        }
    }
}

#[test]
pub(crate) fn call_shared_arguments_reuse_counted_sources_through_receiver_copies() {
    for target in ["&int32", "&int32><null"] {
        let source = format!(
            "f:(a<{target}>){{}};n:1;p:&n;r:{{->p;->tag:true}};alias:f;f(r);r.(alias);out:r.{{copy:$;f(copy)}};outer:(p<&int32>){{r:p.{{->$;->tag:true}};f(((r)))}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(checker.invocations.len(), 4);
        assert_eq!(reports.slot_uses.len(), 4);
        let mut owners = BTreeSet::new();
        for call in checker.invocations.values() {
            let [arg] = call.args[..] else { panic!() };
            let id = projection(&checker, arg);
            let op = &checker.coercions[&id];
            assert_eq!(op.fields, Some(1));
            assert!(matches!(op.source, Some(Shape::SharedScalar(_))));
            assert_eq!(op.kind == CoercionKind::Convert, target.ends_with("<null"));
            let (owner, slot) = reports.slot_uses[&Port::Projection { point: id, step: 0 }];
            assert_eq!((owner, slot.index), (call.owner, 0));
            assert_eq!(checker.points[arg].parent, Some(call.point));
            assert_eq!(checker.bodies[&slot.block].owner, owner);
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
            assert!(!reports.slot_uses.contains_key(&Port::Projection {
                point: call.point,
                step: 0
            }));
            owners.insert(owner);
        }
        assert_eq!(owners.len(), 2);
        assert!(owners.contains(&0));
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
    }
}

#[test]
pub(crate) fn call_argument_hints_preserve_inline_record_and_primary_type_errors() {
    for input in ["{->7;->tag:true}", "(({->7;->tag:true}))"] {
        let source = format!("f:(a<int32>){{}};f({input})");
        let error = crate::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E212");
        assert_eq!(error.message, "scalar result cannot contain field `tag`");
    }
    let error = crate::compile("f:(a<int32>){};r:{->false;->tag:true};f(r)")
        .unwrap_err()
        .remove(0);
    assert_eq!(error.code, "E212");
}

#[test]
pub(crate) fn call_unprojected_arguments_keep_scalar_record_and_borrow_boundaries() {
    for source in [
        "f:(a<int32>){};f(7)",
        "f:(a<int32><null>){};f(7)",
        "f:(a<{-><int32>;tag<boolean>}>){};r:{->7;->tag:true};f(r)",
    ] {
        let (checker, reports) = checked(source);
        let call = checker.invocations.first_key_value().unwrap().1;
        let op = &checker.coercions[&call.args[0]];
        assert!(!op.primary);
        assert!(op.source.is_none() && op.fields.is_none());
        assert!(reports.slot_uses.is_empty());
    }
    for reborrow in [false, true] {
        let source = if reborrow {
            "f:(a<&int32>){};n:=1;p:&!n;f(p)"
        } else {
            "f:(a<&int32>){};n:1;p:&n;f(p)"
        };
        let (checker, reports) = checked(source);
        let call = checker.invocations.first_key_value().unwrap().1;
        let arg = call.args[0];
        assert!(!checker.coercions.contains_key(&arg));
        assert_eq!(checker.reborrow_ops.contains_key(&arg), reborrow);
        assert_eq!(checker.region_edges.contains_key(&arg), !reborrow);
        assert_eq!(checker.points[arg].parent, Some(call.point));
        if reborrow {
            assert!(
                matches!(&reports.effects[&arg].1, Effect::Reborrow(op) if op.acquired && op.result)
            );
        }
        assert!(reports.slot_uses.is_empty());
    }
}
