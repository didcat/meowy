use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true};good==p;bad:p.{->$;->a:true;->z:3};copy:(((bad))~<{-><&int32>;a<boolean>;z<int32>}>);p!=copy";

#[test]
pub(crate) fn shared_local_binaries_reject_late_counts_types_layouts_and_copy_cycles() {
    for fault in 0..15 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.binaries.keys().last().unwrap();
        let port = Port::Projection { point: id, step: 1 };
        let block = reports.slot_uses[&port].1.block;
        let local = checker
            .local_reads
            .iter()
            .find(|(read, op)| {
                checker.points[**read].site == checker.points[id].site
                    && reports.shared_initializers.contains_key(&op.local)
            })
            .unwrap()
            .1
            .local;
        let binding = reports.shared_initializers[&local].init;
        match fault {
            0 | 1 | 2 | 14 => {
                let fields = match fault {
                    0 => [None; 2],
                    1 => [None, Some(1)],
                    2 => [Some(2), None],
                    _ => [None, Some(usize::MAX)],
                };
                let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.fields = fields;
                if fault != 0 {
                    checker.binaries.get_mut(&id).unwrap().fields = fields;
                }
            }
            3 => {
                let types = &mut checker.binaries.get_mut(&id).unwrap().types;
                types.inputs = [BinaryClass::SharedScalar(ScalarKind::Bool); 2];
                let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.types = *types;
            }
            4 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .fields = 1
            }
            5 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .owner += 1
            }
            6 => {
                checker
                    .operations
                    .get_mut(&binding.statement)
                    .unwrap()
                    .shared = None
            }
            7..=9 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
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
pub(crate) fn shared_local_binaries_preserve_earlier_links_when_source_evidence_is_missing() {
    for fault in 0..9 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.binaries.keys().last().unwrap();
        let port = Port::Projection { point: id, step: 1 };
        let block = reports.slot_uses[&port].1.block;
        let (&read, op) = checker
            .local_reads
            .iter()
            .find(|(read, op)| {
                checker.points[**read].site == checker.points[id].site
                    && reports.shared_initializers.contains_key(&op.local)
            })
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
                checker.binaries.get_mut(&id).unwrap().fields = [None; 2];
                let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.fields = [None; 2];
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
pub(crate) fn shared_local_binaries_reject_unrelated_lexical_scope_reads() {
    let source = "n:1;p:&n;a:{r:{->p;->tag:true};r==p};b:{r:{->p;->tag:false};p!=r}";
    let (mut checker, mut reports) = checked(source);
    let (&other, _) = reports.shared_initializers.first_key_value().unwrap();
    let id = *checker.binaries.keys().last().unwrap();
    let (&read, op) = checker
        .local_reads
        .iter_mut()
        .find(|(read, op)| {
            checker.points[**read].site == checker.points[id].site
                && reports.shared_initializers.contains_key(&op.local)
        })
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
