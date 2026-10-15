use super::*;

#[test]
pub(crate) fn counted_binary_lookup_preserves_direct_receiver_evidence_and_diagnostics() {
    for step in 0..2 {
        for fault in 0..15 {
            let (mut checker, mut reports) =
                checked("n:1;p:&n;out:p.{->$;->tag:true}.{a:$==p;b:p!=$}");
            let (&id, op) = checker
                .binaries
                .iter()
                .find(|(_, op)| op.plan.primary[step])
                .unwrap();
            let (input, owner) = (op.inputs[step], op.owner);
            let BinaryClass::SharedScalar(primary) = op.types.inputs[step] else {
                panic!()
            };
            let mut record = SharedRecord {
                primary,
                fields: op.fields[step].unwrap(),
            };
            let slot = reports.slot_uses[&Port::Projection { point: id, step }].1;
            let source = reports.results[&slot.block].1.dispatch.unwrap();
            let (&cell, &(_, receiver, _)) = reports
                .receivers
                .iter()
                .find(|(_, row)| row.2.is_some())
                .unwrap();
            let read = *checker
                .local_reads
                .iter()
                .find(|(read, op)| {
                    op.local == cell && checker.points[**read].site == checker.points[id].site
                })
                .unwrap()
                .0;
            match fault {
                0 => (),
                1 => {
                    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap()
                    else {
                        panic!()
                    };
                    op.result = false;
                }
                2 => {
                    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap()
                    else {
                        panic!()
                    };
                    op.initialized = false;
                }
                3 => {
                    reports.receivers.remove(&cell);
                }
                4 => {
                    reports.effects.remove(&receiver);
                }
                5 => {
                    reports.effects.remove(&read);
                }
                6 => {
                    checker.proofs.mutable.insert(cell);
                }
                7 => record.primary = ScalarKind::Bool,
                8 => record.fields += 1,
                9 => reports.receivers.get_mut(&cell).unwrap().2 = Some(ScalarKind::Bool),
                10 => reports.effects.get_mut(&receiver).unwrap().0 += 1,
                11 => checker.points[input].owner += 1,
                12 => {
                    let Layout::Slots(slots) =
                        &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                    else {
                        panic!()
                    };
                    slots.last_mut().unwrap().mutable = true;
                }
                13 => {
                    checker
                        .bodies
                        .get_mut(&slot.block)
                        .unwrap()
                        .completion
                        .result = Shape::Record { fields: 2 }
                }
                14 => {
                    checker.local_reads.get_mut(&read).unwrap().storage += 1;
                    let (_, Effect::Read { storage, .. }) = reports.effects.get_mut(&read).unwrap()
                    else {
                        panic!()
                    };
                    *storage += 1;
                }
                _ => unreachable!(),
            }
            let (actual, _) = lookup(&mut checker, &reports, input, owner, record);
            match fault {
                0 => assert_eq!(actual.unwrap(), Some(slot)),
                1..=6 => assert_eq!(actual.unwrap(), None, "fault {fault}"),
                _ => {
                    let error = actual.expect_err(&format!("fault {fault}"));
                    assert_eq!(error.code, "B001");
                    assert!(error.message.contains("identity"), "{error:?}");
                }
            }
        }
    }
}

#[test]
pub(crate) fn counted_binary_lookup_bounds_exact_work_for_both_operands() {
    for frame in [
        "a:r==p;b:p!=r",
        "out:r.{a:$==p;b:p!=$}",
        "out:r.{copy:$;a:copy==p;b:p!=copy}",
        "out:r.{copy:=$;a:copy==p;b:p!=copy}",
    ] {
        let source = format!("n:1;p:&n;r:{{->p;->tag:true}};{frame}");
        let (mut checker, reports) = checked(&source);
        let mut seen = [false; 2];
        for op in checker.binaries.values().cloned().collect::<Vec<_>>() {
            for (step, fields) in op.fields.into_iter().enumerate() {
                let Some(fields) = fields else {
                    continue;
                };
                let BinaryClass::SharedScalar(primary) = op.types.inputs[step] else {
                    panic!()
                };
                let record = SharedRecord { primary, fields };
                seen[step] = true;
                let input = op.inputs[step];
                let start = checker.flow.work;
                let (expected, work) = lookup(&mut checker, &reports, input, op.owner, record);
                let expected = expected.unwrap();
                assert!(work > 0);
                let before = format!("{reports:?}{:?}", checker.edge_counts());
                for short in [0, 1] {
                    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
                    checker.flow.full = false;
                    let actual = checker.shared_copies_primary_slot(
                        &reports,
                        input,
                        op.owner,
                        record,
                        Span::default(),
                    );
                    if short == 0 {
                        assert_eq!(actual.unwrap(), expected);
                    } else {
                        assert!(actual.unwrap_err().message.contains("budget"));
                    }
                }
                assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
                checker.flow.work = start;
                checker.flow.full = false;
            }
        }
        assert_eq!(seen, [true; 2]);
    }
}
