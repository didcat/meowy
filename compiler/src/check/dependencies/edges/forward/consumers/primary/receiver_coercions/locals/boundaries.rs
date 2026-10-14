use super::*;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true};a:good.{copy<&int32>:$};bad:p.{->$;->a:true;->z:3};alias:((bad));b:alias.{copy<&int32><null>:((($))~<{-><&int32>;a<boolean>;z<int32>}>) }";

pub(super) fn input_read(checker: &Checker, reports: &Reports, receiver: PointId) -> PointId {
    *checker
        .local_reads
        .iter()
        .find(|(id, op)| {
            reports.shared_initializers.contains_key(&op.local)
                && checker.points[**id].site == checker.points[receiver].site
        })
        .unwrap()
        .0
}

#[test]
pub(crate) fn shared_local_receiver_coercions_reject_stale_counts_referents_layouts_and_cycles() {
    for fault in 0..17 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
            .unwrap()
            .0;
        let slot = reports.slot_uses[&Port::Projection { point: id, step: 0 }].1;
        let receiver = reports.receivers.values().last().unwrap().1;
        let local = checker.local_reads[&input_read(&checker, &reports, receiver)].local;
        let binding = reports.shared_initializers[&local].init;
        match fault {
            0..=2 => {
                let fields = match fault {
                    0 => None,
                    1 => Some(1),
                    _ => Some(usize::MAX),
                };
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.fields = fields;
                if fault != 0 {
                    checker.coercions.get_mut(&id).unwrap().fields = fields;
                }
            }
            3 | 4 => {
                let source = Some(Shape::SharedScalar(if fault == 3 {
                    ScalarKind::Bool
                } else {
                    ScalarKind::Int {
                        bits: 16,
                        signed: false,
                    }
                }));
                checker.coercions.get_mut(&id).unwrap().source = source;
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.source = source;
            }
            5 => {
                checker.dispatch_ops.get_mut(&receiver).unwrap().receiver =
                    Shape::Record { fields: 1 };
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.receiver = Shape::Record { fields: 1 };
            }
            6 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .fields = 1
            }
            7 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .owner += 1
            }
            8 => {
                checker
                    .operations
                    .get_mut(&binding.statement)
                    .unwrap()
                    .shared = None
            }
            9..=11 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    9 => slots[0].shape = Shape::SharedScalar(ScalarKind::Bool),
                    10 => slots[2].mutable = true,
                    11 => slots[2].shape = Shape::SharedScalar(ScalarKind::Bool),
                    _ => unreachable!(),
                }
            }
            12 => {
                reports.index.operations.remove(&binding.statement);
            }
            13 => {
                let input = checker.typed_ops.values().last().unwrap().input;
                checker.group_inputs.get_mut(&input).unwrap().input = input;
            }
            14 => {
                let (&read, op) = checker
                    .local_reads
                    .iter_mut()
                    .find(|(id, op)| {
                        op.local != local
                            && reports.shared_initializers.contains_key(&op.local)
                            && checker.points[**id].site == checker.points[binding.statement].site
                    })
                    .unwrap();
                op.local = local;
                op.storage = local;
                let (
                    _,
                    Effect::Read {
                        local: reported,
                        storage,
                        ..
                    },
                ) = reports.effects.get_mut(&read).unwrap()
                else {
                    panic!()
                };
                *reported = local;
                *storage = local;
            }
            15 => reports.effects.get_mut(&id).unwrap().0 += 1,
            16 => {
                checker
                    .bodies
                    .get_mut(&slot.block)
                    .unwrap()
                    .completion
                    .result = Shape::Record { fields: 1 }
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .slot_uses(&reports, Span::default())
            .expect_err(&format!("fault {fault}"));
        assert!(error.message.contains("identity"), "{error:?}");
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_receiver_coercions_revalidate_receiver_and_initializer_scopes() {
    for receiver in [false, true] {
        let (mut checker, mut reports) = checked(
            "n:1;p:&n;a:{r:{->p;->tag:true};out:r.{copy<&int32>:$}};b:{r:{->p;->tag:false};out:r.{copy<&int32>:$}}",
        );
        let locals: Vec<_> = reports
            .receivers
            .iter()
            .map(|(&local, &(_, dispatch, _))| {
                if receiver {
                    local
                } else {
                    checker.local_reads[&input_read(&checker, &reports, dispatch)].local
                }
            })
            .collect();
        let (other, local) = (locals[0], *locals.last().unwrap());
        let (&id, op) = checker
            .local_reads
            .iter_mut()
            .find(|(_, op)| op.local == local)
            .unwrap();
        op.local = other;
        op.storage = other;
        let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        *local = other;
        *storage = other;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker.slot_uses(&reports, Span::default()).unwrap_err();
        assert!(error.message.contains("scope identity"), "{error:?}");
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
