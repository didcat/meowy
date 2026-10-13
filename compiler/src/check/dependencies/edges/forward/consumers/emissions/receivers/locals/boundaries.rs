use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true};a:good.{copy:{->$}};bad:p.{->$;->z:9;->a:true};alias:((bad));b:alias.{copy:{->((($))~<{-><&int32>;a<boolean>;z<int32>}>)}}";

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
pub(crate) fn shared_local_receiver_emissions_reject_late_descriptors_and_unobserved_suffixes() {
    for fault in 0..15 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&id, op) = checker
            .emissions
            .iter()
            .rev()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let slot = reports.slot_uses[&Port::Emission(op.targets[0].id)].1;
        let receiver = reports.receivers.values().last().unwrap().1;
        let local = checker.local_reads[&input_read(&checker, &reports, receiver)].local;
        let binding = reports.shared_initializers[&local].init;
        let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.initialized = vec![true, false, false];
        op.result = false;
        match fault {
            0 => op.composed.as_mut().unwrap().shared_primary = Some(ScalarKind::Bool),
            1 => {
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
                checker
                    .operations
                    .get_mut(&binding.statement)
                    .unwrap()
                    .shared = None
            }
            5 => {
                checker.dispatch_ops.get_mut(&receiver).unwrap().receiver =
                    Shape::Record { fields: 3 };
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.receiver = Shape::Record { fields: 3 };
            }
            6..=9 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    6 => slots[0].shape = Shape::SharedScalar(ScalarKind::Bool),
                    7 => slots[2].field = Some("wrong".into()),
                    8 => slots[2].mutable = true,
                    9 => slots[2].shape = Shape::SharedScalar(ScalarKind::Bool),
                    _ => unreachable!(),
                }
            }
            10..=12 => {
                let target = &mut op.targets[2];
                match fault {
                    10 => target.field = Some("wrong".into()),
                    11 => target.projection = Projection::Field(0),
                    12 => {
                        target.alias = Some(0);
                        target.storage = Some(0);
                    }
                    _ => unreachable!(),
                }
                checker.emissions.get_mut(&id).unwrap().targets[2] = target.clone();
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
                            && reports.shared_initializers.contains_key(&op.local)
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
        let error = checker
            .slot_uses(&reports, Span::default())
            .expect_err(&format!("fault {fault}"));
        assert!(error.message.contains("identity"), "{error:?}");
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
pub(crate) fn shared_local_receiver_emissions_revalidate_receiver_and_initializer_scopes() {
    for receiver in [false, true] {
        let (mut checker, mut reports) = checked(
            "n:1;p:&n;a:{r:{->p;->tag:true};out:r.{copy:{->$}}};b:{r:{->p;->tag:false};out:r.{copy:{->$}}}",
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
        assert!(
            checker
                .candidate_inputs(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("scope identity")
        );
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("scope identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
