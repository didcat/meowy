use super::{super::super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true;->unused:1};a:{->good.tag};bad:p.{->$;->tag:false;->unused:2};copy:((bad));b:{->copy.tag}";

#[test]
pub(crate) fn shared_local_fields_reject_late_identity_layout_and_cycle_conflicts_atomically() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.fields.keys().last().unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let (&local, row) = reports.shared_initializers.last_key_value().unwrap();
        let statement = row.init.statement;
        match fault {
            0 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .primary = ScalarKind::Bool
            }
            1 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .fields += 1
            }
            2 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .owner += 1
            }
            3 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .input = None
            }
            4 => checker.operations.get_mut(&statement).unwrap().shared = None,
            5 => reports.effects.get_mut(&statement).unwrap().0 += 1,
            6..=8 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    6 => slots[0].shape = Shape::SharedScalar(ScalarKind::Bool),
                    7 => slots.last_mut().unwrap().mutable = true,
                    8 => slots.last_mut().unwrap().shape = Shape::SharedScalar(ScalarKind::Bool),
                    _ => unreachable!(),
                }
            }
            9 => {
                let group = *checker.group_inputs.keys().last().unwrap();
                checker.group_inputs.get_mut(&group).unwrap().input = group;
            }
            10 => {
                let read = *checker
                    .local_reads
                    .iter()
                    .find(|(id, op)| {
                        op.local != local
                            && reports.shared_initializers.contains_key(&op.local)
                            && checker.points[**id].site == checker.points[statement].site
                    })
                    .map(|(id, _)| id)
                    .unwrap();
                let op = checker.local_reads.get_mut(&read).unwrap();
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
            11 => {
                reports.index.operations.remove(&statement);
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
        assert!(
            checker
                .field_results(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_fields_missing_evidence_preserves_earlier_links() {
    for fault in 0..6 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.fields.keys().last().unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let (&local, row) = reports.shared_initializers.last_key_value().unwrap();
        let statement = row.init.statement;
        match fault {
            0 => {
                reports.shared_initializers.remove(&local);
            }
            1 => {
                reports.effects.remove(&statement);
            }
            2 => {
                let read = *checker
                    .local_reads
                    .iter()
                    .find(|(_, op)| op.local == local)
                    .unwrap()
                    .0;
                reports.effects.remove(&read);
            }
            3 => {
                let source = reports.results[&slot.block].1.dispatch.unwrap();
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            4 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .input = None;
                let op = checker.operations.get_mut(&statement).unwrap();
                op.input = None;
                op.edges = vec![Edge::new(
                    Port::Operation(statement),
                    Port::Normal(statement),
                    Route::Next,
                )];
                let (_, Effect::Storage { input, .. }) =
                    reports.effects.get_mut(&statement).unwrap()
                else {
                    panic!()
                };
                *input = None;
            }
            5 => {
                checker.proofs.mutable.insert(local);
            }
            _ => unreachable!(),
        }
        let mut expected = reports.slot_uses.clone();
        expected.remove(&Port::Operation(id));
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
pub(crate) fn shared_local_fields_reject_reads_from_unrelated_lexical_scopes() {
    let source = "n:1;p:&n;a:{r:{->p;->tag:true};->r.tag};b:{r:{->p;->tag:false};->r.tag}";
    let (mut checker, mut reports) = checked(source);
    let (&other, _) = reports.shared_initializers.first_key_value().unwrap();
    let id = *checker.fields.keys().last().unwrap();
    let input = checker.fields[&id].input;
    let read = checker.narrowings.get(&input).map_or(input, |op| op.input);
    let op = checker.local_reads.get_mut(&read).unwrap();
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
