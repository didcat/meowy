use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true};a:{->good};bad:p.{->$;->z:9;->a:true};copy:((bad));b:{->(((copy))~<{-><&int32>;a<boolean>;z<int32>}>) }";

#[test]
pub(crate) fn shared_local_emissions_reject_late_descriptors_and_unobserved_suffixes() {
    for fault in 0..15 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&id, op) = checker
            .emissions
            .iter()
            .rev()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let slot = reports.slot_uses[&Port::Emission(op.targets[0].id)].1;
        let local = checker
            .local_reads
            .iter()
            .find(|(read, _)| checker.points[**read].site == checker.points[id].site)
            .unwrap()
            .1
            .local;
        let binding = reports.shared_initializers[&local].init;
        let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.initialized = vec![true, false, false];
        op.result = false;
        match fault {
            0 | 1 => {
                op.composed.as_mut().unwrap().shared_primary = Some(ScalarKind::Bool);
                if fault == 1 {
                    checker
                        .emissions
                        .get_mut(&id)
                        .unwrap()
                        .composed
                        .as_mut()
                        .unwrap()
                        .shared_primary = Some(ScalarKind::Bool);
                }
            }
            2 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .fields += 1
            }
            3 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .owner += 1
            }
            4 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .input = None
            }
            5 => {
                checker
                    .operations
                    .get_mut(&binding.statement)
                    .unwrap()
                    .shared = None
            }
            6..=8 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    6 => slots[0].shape = Shape::SharedScalar(ScalarKind::Bool),
                    7 => slots[2].field = Some("wrong".into()),
                    8 => slots[2].mutable = true,
                    _ => unreachable!(),
                }
            }
            9..=11 => {
                let target = &mut op.targets[2];
                match fault {
                    9 => target.field = Some("wrong".into()),
                    10 => target.projection = Projection::Field(0),
                    11 => {
                        target.alias = Some(0);
                        target.storage = Some(0);
                    }
                    _ => unreachable!(),
                }
                checker.emissions.get_mut(&id).unwrap().targets[2] = target.clone();
            }
            12 => {
                op.composed.as_mut().unwrap().count += 1;
                checker
                    .emissions
                    .get_mut(&id)
                    .unwrap()
                    .composed
                    .as_mut()
                    .unwrap()
                    .count += 1;
            }
            13 => {
                let input = checker.typed_ops.values().last().unwrap().input;
                checker.group_inputs.get_mut(&input).unwrap().input = input;
            }
            14 => {
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
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.emission_sources,
            checker.edge_counts()
        );
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.emission_sources,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn shared_local_emissions_preserve_earlier_links_when_evidence_is_missing() {
    for fault in 0..9 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&id, op) = checker
            .emissions
            .iter()
            .rev()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let ports: BTreeSet<_> = op
            .targets
            .iter()
            .map(|target| Port::Emission(target.id))
            .collect();
        let slot = reports.slot_uses[&Port::Emission(op.targets[0].id)].1;
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
                reports.results.remove(&slot.block);
            }
            4 => {
                reports
                    .effects
                    .remove(checker.typed_ops.keys().last().unwrap());
            }
            5 => {
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.composed.as_mut().unwrap().shared_primary = None;
                checker
                    .emissions
                    .get_mut(&id)
                    .unwrap()
                    .composed
                    .as_mut()
                    .unwrap()
                    .shared_primary = None;
            }
            6 => {
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.initialized.fill(false);
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
        let expected: Uses = reports
            .slot_uses
            .iter()
            .filter(|(port, _)| !ports.contains(port))
            .map(|(&port, &slot)| (port, slot))
            .collect();
        assert_eq!(expected.len(), 2);
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
pub(crate) fn shared_local_emissions_reject_reads_from_unrelated_scopes() {
    let source =
        "n:1;p:&n;a:{r:{->p;->a:true;->z:9};out:{->r}};b:{r:{->p;->a:false;->z:8};out:{->r}}";
    let (mut checker, mut reports) = checked(source);
    let (&other, _) = reports.shared_initializers.first_key_value().unwrap();
    let (&id, _) = checker
        .emissions
        .iter()
        .rev()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
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
