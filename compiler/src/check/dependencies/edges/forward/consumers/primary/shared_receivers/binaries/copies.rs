use super::*;

mod boundaries;
mod evidence;
mod limits;
mod stages;

pub(super) fn copy_read(checker: &Checker, reports: &Reports, id: PointId) -> PointId {
    *checker
        .local_reads
        .iter()
        .find(|(read, op)| {
            checker.points[**read].site == checker.points[id].site
                && reports.shared_initializers.contains_key(&op.local)
        })
        .unwrap()
        .0
}

#[test]
pub(crate) fn receiver_copy_binaries_retain_operand_ownership_across_scopes() {
    for input in ["$", "copy", "alias"] {
        for init in ["{->p;->a:true;->z:3}", "p.{->$;->a:true;->z:3}"] {
            let source = format!(
                "n:1;p:&n;flag:=true;r:{init};out:r.{{copy:$;alias:((copy)~<{{-><&int32>;a<boolean>;z<int32>}}>);a:{input}==p;b:p!={input};same:copy==copy;|flag|inner:copy.{{next:$;->next==p}};value:{{->copy.a}};joined:{{->copy}}}};f<boolean>:(p<&int32>){{r:{init};->r.{{copy:$;alias:copy;->{input}==p}}}}"
            );
            let (mut checker, reports) = checked(&source);
            let mut owned = 0;
            let mut linked = 0;
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
                    assert_eq!(op.fields[step], primary.then_some(2));
                    if !primary {
                        continue;
                    }
                    let BinaryClass::SharedScalar(kind) = op.types.inputs[step] else {
                        panic!()
                    };
                    let record = crate::check::dependencies::operations::SharedRecord {
                        primary: kind,
                        fields: op.fields[step].unwrap(),
                    };
                    let block = checker
                        .shared_copy_body(
                            &reports,
                            op.inputs[step],
                            op.owner,
                            record,
                            Span::default(),
                        )
                        .unwrap()
                        .unwrap();
                    let port = Port::Projection { point: id, step };
                    if let Some(&(owner, slot)) = reports.slot_uses.get(&port) {
                        assert_eq!((owner, slot.block, slot.index), (op.owner, block, 0));
                        linked += 1;
                    }
                    let Layout::Slots(layout) = &checker.bodies[&block].layout else {
                        panic!()
                    };
                    assert_eq!(layout.len(), 3);
                    assert_eq!(layout[0].shape, Shape::SharedScalar(kind));
                    assert_eq!(
                        reports.results[&block].1.slots.as_ref().unwrap()[0],
                        Sources::Unknown
                    );
                    owners.insert(op.owner);
                    owned += 1;
                }
                if op.types.inputs == [BinaryClass::Record { fields: 2 }; 2] {
                    assert_eq!(op.plan.primary, [false; 2]);
                    records += 1;
                }
            }
            assert_eq!((owned, records), (4, 1));
            assert_eq!(linked, 4);
            assert_eq!(owners, std::collections::BTreeSet::from([0, 1]));
            let coercions: Vec<_> = checker
                .coercions
                .iter()
                .filter(|(_, op)| op.primary)
                .collect();
            assert!(coercions.is_empty());
            assert_eq!(reports.field_results.len(), 1);
            assert_eq!(
                reports
                    .slot_uses
                    .keys()
                    .filter(|port| matches!(port, Port::Emission(_)))
                    .count(),
                3
            );
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
        }
    }
}

#[test]
pub(crate) fn receiver_copy_binaries_leave_ascribed_operand_projections_with_coercions() {
    for tail in [
        "(copy~<{-><&int32>;tag<boolean>}>)!=p",
        "p!=(copy~<{-><&int32>;tag<boolean>}>)",
    ] {
        let source = format!("n:1;p:&n;r:{{->p;->tag:true}};out:r.{{copy:$;->{tail}}}");
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
        assert_eq!(checker.coercions[&point].fields, Some(1));
    }
}

#[test]
pub(crate) fn receiver_copy_binaries_capture_stopped_operand_prefixes() {
    for tail in ["copy==d.panic(\"stop\")", "d.panic(\"stop\")==copy"] {
        let source =
            format!("d:@\"debug\";n:1;p:&n;r:{{->p;->tag:true}};out:r.{{copy:$;->{tail}}}");
        let (checker, reports) = checked(&source);
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        if tail.starts_with('d') {
            assert!(!reports.effects.contains_key(&id));
            assert!(
                !reports.entries[&0]
                    .1
                    .ports
                    .contains(&Port::Entry(op.inputs[1]))
            );
        } else {
            assert_eq!(op.plan.primary, [true, false]);
            assert_eq!(op.fields, [Some(1), None]);
            let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(observed.projected, [true, false]);
            assert!(!observed.operation && !observed.result);
        }
        assert_eq!(reports.slot_uses.len(), usize::from(!tail.starts_with('d')));
    }
}
