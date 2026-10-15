use super::*;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true};a:good.{copy:$;cmp:copy==p};bad:p.{->$;->a:true;->z:3};root:((bad));b:root.{copy:(($));alias:((copy)~<{-><&int32>;a<boolean>;z<int32>}>);cmp:p!=alias}";

#[test]
pub(crate) fn receiver_copy_binaries_reject_stale_counts_referents_layouts_and_cycles() {
    for (tail, step) in [("alias!=p", 0), ("p!=alias", 1)] {
        let source = SOURCE.replace("p!=alias", tail);
        for fault in 0..24 {
            let (mut checker, mut reports) = checked(&source);
            let id = *checker.binaries.keys().last().unwrap();
            let slot = reports.slot_uses[&Port::Projection { point: id, step }].1;
            let (&cell, &(_, receiver, _)) = reports.receivers.last_key_value().unwrap();
            let read = copy_read(&checker, &reports, id);
            let local = checker.local_reads[&read].local;
            let binding = reports.shared_initializers[&local].init;
            let copy = checker.local_reads[&copy_read(&checker, &reports, binding.statement)].local;
            let copied = reports.shared_initializers[&copy].init;
            match fault {
                0..=2 => {
                    let fields = match fault {
                        0 => None,
                        1 => Some(1),
                        _ => Some(usize::MAX),
                    };
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.fields[step] = fields;
                    if fault != 0 {
                        checker.binaries.get_mut(&id).unwrap().fields[step] = fields;
                    }
                }
                3 | 4 => {
                    let kind = if fault == 3 {
                        ScalarKind::Bool
                    } else {
                        ScalarKind::Int {
                            bits: 16,
                            signed: false,
                        }
                    };
                    let types = &mut checker.binaries.get_mut(&id).unwrap().types;
                    types.inputs = [BinaryClass::SharedScalar(kind); 2];
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.types = *types;
                }
                5 => {
                    checker.dispatch_ops.get_mut(&receiver).unwrap().receiver =
                        Shape::Record { fields: 1 };
                    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap()
                    else {
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
                    let Layout::Slots(slots) =
                        &mut checker.bodies.get_mut(&slot.block).unwrap().layout
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
                                && checker.points[**id].site
                                    == checker.points[binding.statement].site
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
                17 => {
                    reports
                        .shared_initializers
                        .get_mut(&copy)
                        .unwrap()
                        .record
                        .fields += 1
                }
                18 => {
                    checker
                        .operations
                        .get_mut(&copied.statement)
                        .unwrap()
                        .shared = None
                }
                19 => {
                    checker.local_reads.get_mut(&read).unwrap().storage = copy;
                    let (_, Effect::Read { storage, .. }) = reports.effects.get_mut(&read).unwrap()
                    else {
                        panic!()
                    };
                    *storage = copy;
                }
                20 => {
                    let original = *checker
                        .local_reads
                        .iter()
                        .find(|(id, op)| {
                            op.local == cell
                                && checker.points[**id].site
                                    == checker.points[copied.statement].site
                        })
                        .unwrap()
                        .0;
                    let op = checker.local_reads.get_mut(&original).unwrap();
                    op.local = copy;
                    op.storage = copy;
                    let (_, Effect::Read { local, storage, .. }) =
                        reports.effects.get_mut(&original).unwrap()
                    else {
                        panic!()
                    };
                    *local = copy;
                    *storage = copy;
                }
                21 => {
                    checker.binaries.get_mut(&id).unwrap().fields[1 - step] = Some(2);
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.fields[1 - step] = Some(2);
                }
                22 => {
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.inputs.swap(0, 1);
                }
                23 => {
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.projected[1 - step] = true;
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
}

#[test]
pub(crate) fn receiver_copy_binaries_revalidate_original_local_and_receiver_scopes() {
    for receiver in [false, true] {
        let (mut checker, mut reports) = checked(
            "n:1;p:&n;r:{->p;->tag:true};a:r.{copy:$;cmp:copy==p};b:r.{copy:$;cmp:copy==p}",
        );
        let ids: Vec<_> = checker.binaries.keys().copied().collect();
        let reads: Vec<_> = ids
            .iter()
            .map(|id| copy_read(&checker, &reports, *id))
            .collect();
        let (id, other) = if receiver {
            let (&cell, _) = reports.receivers.last_key_value().unwrap();
            (
                *checker
                    .local_reads
                    .iter()
                    .find(|(_, op)| op.local == cell)
                    .unwrap()
                    .0,
                *reports.receivers.keys().next().unwrap(),
            )
        } else {
            (reads[1], checker.local_reads[&reads[0]].local)
        };
        let op = checker.local_reads.get_mut(&id).unwrap();
        op.local = other;
        op.storage = other;
        let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        *local = other;
        *storage = other;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("scope identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    let (mut checker, mut reports) =
        checked("n:1;p:&n;r:{->p;->tag:true};out:r.{copy:$;inner:3.{cmp:p!=copy}}");
    assert_eq!(reports.slot_uses.len(), 1);
    let cells: Vec<_> = reports.receivers.keys().copied().collect();
    let (&id, op) = checker
        .local_reads
        .iter_mut()
        .find(|(_, op)| op.local == cells[0])
        .unwrap();
    op.local = cells[1];
    op.storage = cells[1];
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    *local = cells[1];
    *storage = cells[1];
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("receiver-scope identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
