use super::*;
use crate::check::dependencies::{edges::forward::results::Sources, operations::SharedRecord};

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
pub(crate) fn receiver_copy_coercions_retain_owned_projection_descriptors_across_scopes() {
    for input in ["$", "copy", "alias"] {
        for init in ["{->p;->tag:true;->n:3}", "p.{->$;->tag:true;->n:3}"] {
            for target in ["&int32", "&int32><null"] {
                let source = format!(
                    "n:1;p:&n;r:{init};flag:=true;out:r.{{copy:$;alias:((copy)~<{{-><&int32>;tag<boolean>;n<int32>}}>);a<{target}>:{input};|flag|inner:copy.{{next:$;b<{target}>:next}};value:{{->copy.tag}};joined:{{->copy}};cmp:copy==p}};f<null>:(p<&int32>){{r:{init};out:r.{{copy:$;alias:copy;c<{target}>:{input}}}}}"
                );
                let (mut checker, reports) = checked(&source);
                let mut owners = BTreeSet::new();
                let mut linked = 0;
                let mut count = 0;
                for (&id, op) in checker.coercions.iter().filter(|(_, op)| op.primary) {
                    let (_, Effect::Coercion(observed)) = &reports.effects[&id] else {
                        panic!()
                    };
                    assert_eq!(op.fields, Some(2));
                    assert_eq!(observed.fields, op.fields);
                    assert_eq!(observed.source, op.source);
                    assert!(observed.projected && observed.result);
                    assert_eq!(observed.operation, op.kind == CoercionKind::Convert);
                    assert_eq!(op.kind == CoercionKind::Convert, target.ends_with("<null"));
                    let port = Port::Projection { point: id, step: 0 };
                    if let Some(&(owner, slot)) = reports.slot_uses.get(&port) {
                        assert_eq!((owner, slot.index), (op.owner, 0));
                        let Layout::Slots(layout) = &checker.bodies[&slot.block].layout else {
                            panic!()
                        };
                        assert_eq!(layout.len(), 3);
                        assert_eq!(op.source, Some(layout[0].shape));
                        assert_eq!(
                            reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                            Sources::Unknown
                        );
                        linked += 1;
                    }
                    owners.insert(op.owner);
                    count += 1;
                }
                assert_eq!(count, 3);
                assert_eq!(linked, 3);
                assert_eq!(owners, BTreeSet::from([0, 1]));
                assert_eq!(reports.field_results.len(), 1);
                assert_eq!(
                    reports
                        .slot_uses
                        .keys()
                        .filter(|port| matches!(port, Port::Emission(_)))
                        .count(),
                    3
                );
                assert!(checker.binaries.values().any(|op| op.plan.primary[0]));
                assert!(!reports.slot_uses.keys().any(|port| matches!(port, Port::Projection { point, .. } if checker.binaries.contains_key(point))));
                assert_eq!(
                    checker.slot_uses(&reports, Span::default()).unwrap(),
                    reports.slot_uses
                );
            }
        }
    }
}

#[test]
pub(crate) fn receiver_copy_coercions_keep_the_binary_primary_selector_separate() {
    let (mut checker, reports) =
        checked("n:1;p:&n;r:{->p;->tag:true};out:r.{copy:$;view<&int32>:copy}");
    let op = checker.coercions.values().find(|op| op.primary).unwrap();
    let (input, owner) = (op.input, op.owner);
    let Some(Shape::SharedScalar(primary)) = op.source else {
        panic!()
    };
    let record = SharedRecord {
        primary,
        fields: op.fields.unwrap(),
    };
    assert_eq!(
        checker
            .shared_record_primary_slot(&reports, input, owner, record, Span::default())
            .unwrap(),
        None
    );
    let block = checker
        .shared_copy_body(&reports, input, owner, record, Span::default())
        .unwrap()
        .unwrap();
    assert_eq!(
        reports.results[&block].1.slots.as_ref().unwrap()[0],
        Sources::Unknown
    );
    for &local in reports.shared_initializers.keys() {
        assert!(!reports.eligible.contains(&local));
        assert!(!reports.initializers.contains_key(&local));
        assert!(!reports.receivers.contains_key(&local));
    }
}
