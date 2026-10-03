use super::{super::super::tests::checked, *};

pub(super) fn source() -> &'static str {
    "source:{->7;->a:1;->b:true};copy:'out{{'out->source}}"
}

#[test]
pub(crate) fn emission_reports_preserve_independent_visits_without_destination_results() {
    let (mut checker, mut reports) = checked(source(), false);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
    let owner = op.owner;
    let Effect::Emission(full) = reports.effects[&id].1.clone() else {
        panic!()
    };
    reports.blocks.remove(&op.targets[0].block);
    for state in 0..5 {
        let mut observed = full.clone();
        observed.initialized.fill(false);
        observed.result = state == 3;
        if state < 3 {
            observed.initialized[state] = true;
        }
        let before = format!("{reports:?}");
        checker
            .validate_emission_report(&reports, id, owner, &observed, Span::default())
            .unwrap();
        assert_eq!(format!("{reports:?}"), before);
    }
}

#[test]
pub(crate) fn emission_reports_reject_metadata_and_unobserved_edge_corruption_atomically() {
    for fault in 0..17 {
        let (mut checker, reports) = checked(source(), false);
        let (&id, op) = checker
            .emissions
            .iter()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let mut owner = op.owner;
        let Effect::Emission(mut observed) = reports.effects[&id].1.clone() else {
            panic!()
        };
        observed.initialized.fill(false);
        observed.initialized[0] = true;
        observed.result = false;
        match fault {
            0 => owner += 1,
            1 => observed.input = id,
            2 => observed.composed = None,
            3 => observed.composed.as_mut().unwrap().local += 1,
            4 => observed.composed.as_mut().unwrap().count += 1,
            5 => observed.targets.clear(),
            6 => observed.targets[2].id += 1,
            7 => observed.targets[2].block += 1,
            8 => observed.targets[2].field = Some("other".into()),
            9 => observed.targets[2].projection = Projection::Value,
            10 => observed.targets[2].alias = Some(0),
            11 => observed.targets[2].storage = Some(0),
            12 => observed.control = !observed.control,
            13 => observed.initialized.clear(),
            14 => {
                checker.emissions.get_mut(&id).unwrap().edges.pop();
            }
            15 => {
                checker.emission_sources.remove(&observed.targets[2].id);
            }
            16 => {
                checker.emissions.remove(&id);
            }
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.emissions, checker.emission_sources
        );
        assert!(
            checker
                .validate_emission_report(&reports, id, owner, &observed, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.emissions, checker.emission_sources
            ),
            before
        );
    }
}

#[test]
pub(crate) fn emission_reports_replay_once_with_exact_work_and_descriptor_bounds() {
    let (mut checker, reports) = checked(source(), false);
    let (&id, op) = checker
        .emissions
        .iter()
        .find(|(_, op)| op.composed.is_some())
        .unwrap();
    let owner = op.owner;
    let Effect::Emission(mut observed) = reports.effects[&id].1.clone() else {
        panic!()
    };
    let before = checker.flow.work;
    checker
        .emission_effect_stage(&reports, owner, Port::Normal(id), Span::default())
        .unwrap();
    let replay = checker.flow.work - before;
    let before = checker.flow.work;
    checker
        .validate_emission_report(&reports, id, owner, &observed, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    assert_eq!(work, replay + observed.targets.len() * 4 + 11);
    let before = format!("{reports:?}");
    for missing in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + missing;
        assert_eq!(
            checker
                .validate_emission_report(&reports, id, owner, &observed, Span::default())
                .is_ok(),
            missing == 0
        );
        assert_eq!(format!("{reports:?}"), before);
    }
    for names in [false, true] {
        checker.flow = crate::flow::Flow::new();
        if names {
            observed.targets.truncate(3);
            observed.targets[2].field = Some("x".repeat(MAX_EDGES + 1));
        } else {
            observed
                .targets
                .resize(MAX_TARGETS + 1, observed.targets[0].clone());
        }
        assert!(
            checker
                .validate_emission_report(&reports, id, owner, &observed, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(format!("{reports:?}"), before);
    }
}
