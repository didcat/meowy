use super::*;

mod boundaries;
mod evidence;
mod limits;
mod stages;

#[test]
pub(crate) fn shared_local_receiver_binaries_preserve_ascribed_operand_coercion_ownership() {
    for tail in [
        "($~<{-><&int32>;a<boolean>;z<int32>}>)!=p",
        "p!=($~<{-><&int32>;a<boolean>;z<int32>}>)",
    ] {
        let source = format!("n:1;p:&n;r:{{->p;->a:true;->z:3}};out:r.{{->{tail}}}");
        let (checker, reports) = checked(&source);
        assert!(
            checker
                .binaries
                .values()
                .all(|op| op.plan.primary == [false; 2] && op.fields == [None; 2])
        );
        assert_eq!(reports.slot_uses.len(), 1);
        let (&Port::Projection { point, step: 0 }, _) =
            reports.slot_uses.first_key_value().unwrap()
        else {
            panic!()
        };
        assert!(checker.coercions[&point].primary);
        assert_eq!(checker.coercions[&point].fields, Some(2));
    }
}

#[test]
pub(crate) fn shared_local_receiver_binaries_link_owned_operands_without_duplicate_coercions() {
    for init in ["{->p;->tag:true}", "p.{->$;->tag:true}"] {
        for stored in [false, true] {
            let prefix = if stored {
                format!("r:{init};alias:((r));")
            } else {
                String::new()
            };
            let input = if stored { "alias" } else { init };
            let source = format!(
                "n:1;p:&n;flag:=true;{prefix}out:({input}).{{a:$==p;b:p!=(($));same:$==$;|flag|inner:$.{{->$!=p}};value:$.tag;copy:{{->$}}}};f<boolean>:(p<&int32>){{{prefix}->({input}).{{->$==p}}}}"
            );
            let (mut checker, reports) = checked(&source);
            let mut owned = 0;
            let mut records = 0;
            let mut owners = std::collections::BTreeSet::new();
            for (&id, op) in &checker.binaries.clone() {
                let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
                    panic!()
                };
                assert_eq!(observed.projected, op.plan.primary);
                assert_eq!(observed.fields, op.fields);
                let inputs = checker
                    .primary_effect_inputs(
                        &reports,
                        id,
                        op.owner,
                        &reports.effects[&id].1,
                        Span::default(),
                    )
                    .unwrap();
                for (step, primary) in op.plan.primary.into_iter().enumerate() {
                    assert_eq!(inputs[step], primary.then_some(op.inputs[step]));
                    assert_eq!(op.fields[step], primary.then_some(1));
                    if primary {
                        assert_eq!(step, 0);
                        let port = Port::Projection { point: id, step };
                        let (owner, slot) = reports.slot_uses[&port];
                        assert_eq!((owner, slot.index), (op.owner, 0));
                        let BinaryClass::SharedScalar(kind) = op.types.inputs[step] else {
                            panic!()
                        };
                        let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                            panic!()
                        };
                        assert_eq!(layout.len(), op.fields[step].unwrap() + 1);
                        assert_eq!(layout[0].shape, Shape::SharedScalar(kind));
                        assert_eq!(
                            reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                            Sources::Unknown
                        );
                        owners.insert(op.owner);
                        owned += 1;
                    }
                }
                if op.types.inputs == [BinaryClass::Record { fields: 1 }; 2] {
                    assert_eq!(op.plan.primary, [false; 2]);
                    records += 1;
                }
            }
            assert_eq!((owned, records), (3, 1));
            assert_eq!(owners, std::collections::BTreeSet::from([0, 1]));
            let coercions: Vec<_> = checker
                .coercions
                .iter()
                .filter(|(_, op)| op.primary)
                .collect();
            assert_eq!(coercions.len(), 1);
            assert!(reports.slot_uses.contains_key(&Port::Projection {
                point: *coercions[0].0,
                step: 0
            }));
            assert_eq!(reports.field_results.len(), 1);
            assert_eq!(
                reports
                    .slot_uses
                    .keys()
                    .filter(|port| matches!(port, Port::Emission(_)))
                    .count(),
                2
            );
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
        }
    }
}

#[test]
pub(crate) fn shared_local_receiver_binaries_keep_receiver_copies_and_full_records_opaque() {
    for source in [
        "n:1;p:&n;r:{->p;->tag:true};out:r.{copy:$;->copy==p}",
        "n:1;p:&n;r:{->p;->tag:true};out:r.{copy:$;inner:copy.{->$==p}}",
        "n:1;p:&n;r:{->p;->tag:true};out:r.{->$==$}",
        "n:1;p:&n;r:={->p;->tag:true};out:r.{->$==p}",
        "n:1;p:&n;r:{->p;->tag:=true};out:r.{->$==p}",
        "n:1;p:&n;r:{->p;->other:p};out:r.{->$==p}",
        "n:1;p:&n;r:{->p;->tag:true};out:(&r).{->(*$)==p}",
        "f<boolean>:(r<{-><&int32>;tag<boolean>}>,p<&int32>){->r.{->$==p}}",
        "f<{-><&int32>;tag<boolean>}>:(p<&int32>){->p;->tag:true};n:1;p:&n;out:f(p).{->$==p}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
