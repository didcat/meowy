use super::*;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true};a<&int32>:good;bad:p.{->$;->a:true;->z:3};copy:((bad));b<&int32><null>:(((copy))~<{-><&int32>;a<boolean>;z<int32>}>)";

#[test]
pub(crate) fn shared_local_coercions_reject_late_counts_types_layouts_and_cycles_atomically() {
    for fault in 0..15 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
            .unwrap()
            .0;
        let port = Port::Projection { point: id, step: 0 };
        let slot = reports.slot_uses[&port].1;
        let local = checker
            .local_reads
            .iter()
            .find(|(read, _)| checker.points[**read].site == checker.points[id].site)
            .unwrap()
            .1
            .local;
        let binding = reports.shared_initializers[&local].init;
        match fault {
            0 | 1 | 14 => {
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
            2 => {
                let source = Some(Shape::SharedScalar(ScalarKind::Bool));
                checker.coercions.get_mut(&id).unwrap().source = source;
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.source = source;
            }
            3 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .fields = 1
            }
            4 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .owner += 1
            }
            5 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .input = None
            }
            6 => {
                checker
                    .operations
                    .get_mut(&binding.statement)
                    .unwrap()
                    .shared = None
            }
            7..=9 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    7 => slots[0].shape = Shape::SharedScalar(ScalarKind::Bool),
                    8 => slots[2].mutable = true,
                    9 => slots[2].shape = Shape::SharedScalar(ScalarKind::Bool),
                    _ => unreachable!(),
                }
            }
            10 => reports.effects.get_mut(&id).unwrap().0 += 1,
            11 => {
                reports.index.operations.remove(&binding.statement);
            }
            12 => {
                let input = checker.typed_ops.values().last().unwrap().input;
                checker.group_inputs.get_mut(&input).unwrap().input = input;
            }
            13 => {
                let (&read, op) = checker
                    .local_reads
                    .iter_mut()
                    .find(|(read, op)| {
                        op.local != local
                            && checker.points[**read].site == checker.points[binding.statement].site
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
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_coercions_preserve_earlier_links_when_source_evidence_is_missing() {
    for fault in 0..9 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
            .unwrap()
            .0;
        let port = Port::Projection { point: id, step: 0 };
        let block = reports.slot_uses[&port].1.block;
        let (&read, op) = checker
            .local_reads
            .iter()
            .find(|(read, _)| checker.points[**read].site == checker.points[id].site)
            .unwrap();
        let local = op.local;
        let binding = reports.shared_initializers[&local].init;
        match fault {
            0 => {
                reports.shared_initializers.remove(&local);
            }
            1 => {
                reports.effects.remove(&binding.statement);
            }
            2 => {
                reports.effects.remove(&read);
            }
            3 => {
                reports.results.remove(&block);
            }
            4 => {
                checker.coercions.get_mut(&id).unwrap().fields = None;
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.fields = None;
            }
            5 => {
                reports
                    .effects
                    .remove(checker.typed_ops.keys().last().unwrap());
            }
            6 => {
                let source = reports.results[&block].1.dispatch.unwrap();
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            7 => {
                checker.proofs.mutable.insert(local);
            }
            8 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .input = None;
                let op = checker.operations.get_mut(&binding.statement).unwrap();
                op.input = None;
                op.edges = vec![Edge::new(
                    Port::Operation(binding.statement),
                    Port::Normal(binding.statement),
                    Route::Next,
                )];
                let (_, Effect::Storage { input, .. }) =
                    reports.effects.get_mut(&binding.statement).unwrap()
                else {
                    panic!()
                };
                *input = None;
            }
            _ => unreachable!(),
        }
        let mut expected = reports.slot_uses.clone();
        expected.remove(&port);
        assert_eq!(expected.len(), 1);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected,
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_coercions_reject_unrelated_lexical_scope_reads() {
    let source =
        "n:1;p:&n;a:{r:{->p;->tag:true};copy<&int32>:r};b:{r:{->p;->tag:false};copy<&int32>:r}";
    let (mut checker, mut reports) = checked(source);
    let (&other, _) = reports.shared_initializers.first_key_value().unwrap();
    let id = *checker
        .coercions
        .iter()
        .rev()
        .find(|(_, op)| op.primary)
        .unwrap()
        .0;
    let (&read, op) = checker
        .local_reads
        .iter_mut()
        .find(|(read, _)| checker.points[**read].site == checker.points[id].site)
        .unwrap();
    op.local = other;
    op.storage = other;
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&read).unwrap() else {
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
