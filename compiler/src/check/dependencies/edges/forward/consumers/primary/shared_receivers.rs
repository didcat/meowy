use super::{super::tests::checked, *};

mod binaries;

#[test]
pub(crate) fn shared_receiver_primaries_qualify_exact_sources_groups_scopes_and_owners() {
    for init in ["{->p;->tag:true}", "p.{->$;->tag:true}"] {
        let source = format!(
            "n:1;p:&n;flag:=true;x:({init}).{{a:(( $ ))==p;|flag|b:$!=p;inner:$.{{->$==p}};copy<&int32>:($~<{{-><&int32>;tag<boolean>}}> )}};f<boolean>:(p<&int32>){{->({init}).{{->$==p}}}}"
        );
        let (mut checker, reports) = checked(&source);
        let ops: Vec<_> = checker.binaries.values().cloned().collect();
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut count = 0;
        for op in ops {
            for (step, input) in op.inputs.into_iter().enumerate() {
                if !op.plan.primary[step] {
                    continue;
                }
                let BinaryClass::SharedScalar(kind) = op.types.inputs[step] else {
                    panic!()
                };
                let slot = checker
                    .receiver_primary_shape(
                        &reports,
                        input,
                        op.owner,
                        Shape::SharedScalar(kind),
                        Span::default(),
                    )
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    (slot.index, checker.bodies[&slot.block].owner),
                    (0, op.owner)
                );
                let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(slots[0].shape, Shape::SharedScalar(kind));
                assert_eq!(
                    checker
                        .record_receiver_body(&reports, input, op.owner, Span::default())
                        .unwrap(),
                    None
                );
                count += 1;
            }
        }
        assert_eq!(count, 4);
        let coercions: Vec<_> = checker
            .coercions
            .values()
            .filter(|op| matches!(op.source, Some(Shape::SharedScalar(_))))
            .cloned()
            .collect();
        assert_eq!(coercions.len(), 1);
        for op in coercions {
            assert!(
                checker
                    .receiver_primary_shape(
                        &reports,
                        op.input,
                        op.owner,
                        op.source.unwrap(),
                        Span::default()
                    )
                    .unwrap()
                    .is_some()
            );
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_receiver_primaries_reject_type_conflicts_and_keep_ineligible_sources_opaque() {
    let source = "n:1;p:&n;x:{->p;->tag:true}.{->$==p}";
    for fault in 0..3 {
        let (mut checker, reports) = checked(source);
        let op = checker.binaries.values().next().unwrap().clone();
        let mut shape = Shape::SharedScalar(ScalarKind::Int {
            bits: 32,
            signed: true,
        });
        if fault == 0 {
            shape = Shape::SharedScalar(ScalarKind::Bool);
        } else {
            let block = checker
                .shared_receiver_body(
                    &reports,
                    op.inputs[0],
                    op.owner,
                    ScalarKind::Int {
                        bits: 32,
                        signed: true,
                    },
                    Span::default(),
                )
                .unwrap()
                .unwrap();
            let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout else {
                panic!()
            };
            if fault == 1 {
                slots[0].shape = Shape::SharedScalar(ScalarKind::Bool);
            } else {
                slots[0].mutable = true;
            }
        }
        assert!(
            checker
                .receiver_primary_shape(&reports, op.inputs[0], op.owner, shape, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    for init in ["r", "{->p;->tag:=true}", "{->p;->other:p}"] {
        let source = format!("n:1;p:&n;r:{{->p;->tag:true}};x:({init}).{{->$==p}}");
        let (mut checker, reports) = checked(&source);
        let op = checker.binaries.values().next().unwrap().clone();
        assert_eq!(
            checker
                .receiver_primary_shape(
                    &reports,
                    op.inputs[0],
                    op.owner,
                    Shape::SharedScalar(ScalarKind::Int {
                        bits: 32,
                        signed: true
                    }),
                    Span::default()
                )
                .unwrap(),
            None
        );
    }
}
