use super::{super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;out:p.{->$;->z:9;->a:true}.{copy:{->(($))}}";

#[test]
pub(crate) fn shared_emission_reports_keep_exact_types_and_independent_visits_at_fixed_cost() {
    let mut costs = Vec::new();
    for value in ["n", "&n"] {
        let (mut checker, mut reports) =
            checked(&SOURCE.replace("p:&n", &format!("p:{value}")), false);
        let (&id, op) = checker
            .emissions
            .iter()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let composed = op.composed;
        let ports: Vec<_> = op
            .targets
            .iter()
            .map(|target| Port::Emission(target.id))
            .chain([Port::Normal(id)])
            .collect();
        for (index, port) in ports.into_iter().enumerate() {
            reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            let start = checker.flow.work;
            let effects = checker
                .operation_effects_limited(&reports, Span::default(), 1, 8, 0)
                .unwrap();
            let work = checker.flow.work - start;
            costs.push(work);
            let (_, Effect::Emission(observed)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(observed.composed, composed);
            assert_eq!(
                observed.initialized,
                (0..3).map(|part| part == index).collect::<Vec<_>>()
            );
            assert_eq!(observed.result, index == 3);
            for (limit, parts, short, pass) in [
                (1, 8, 0, true),
                (0, 8, 0, false),
                (1, 7, 0, false),
                (1, 8, 1, false),
            ] {
                checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
                checker.flow.full = false;
                let result =
                    checker.operation_effects_limited(&reports, Span::default(), limit, parts, 0);
                if pass {
                    assert_eq!(result.unwrap(), effects);
                } else {
                    assert!(result.unwrap_err().message.contains("budget"));
                }
                assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
            }
            checker.flow.work = 0;
            checker.flow.full = false;
        }
    }
    assert_eq!(costs[..4], costs[4..]);
}

#[test]
pub(crate) fn shared_emission_reports_reject_stale_referents_without_mutating_prior_visits() {
    for primary in [
        None,
        Some(ScalarKind::Bool),
        Some(ScalarKind::Int {
            bits: 16,
            signed: true,
        }),
        Some(ScalarKind::Int {
            bits: 32,
            signed: false,
        }),
    ] {
        let (mut checker, reports) = checked(SOURCE, false);
        let (&id, op) = checker
            .emissions
            .iter()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let owner = op.owner;
        let mut effects = reports.effects.clone();
        let (_, Effect::Emission(observed)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        observed.composed.as_mut().unwrap().shared_primary = primary;
        observed.initialized = vec![true, false, false];
        observed.result = false;
        assert!(
            checker
                .validate_emission_report(&reports, id, owner, observed, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        let before = effects.clone();
        let mut parts = 0;
        assert!(
            checker
                .record_emission_effect(
                    owner,
                    (id, Some(2)),
                    &mut effects,
                    usize::MAX,
                    &mut parts,
                    Span::default()
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
        assert_eq!(parts, 0);
    }
}

#[test]
pub(crate) fn shared_emission_reports_reject_invalid_descriptors_before_publication() {
    for primary in [
        ScalarKind::Int {
            bits: 7,
            signed: true,
        },
        ScalarKind::Float { bits: 16 },
    ] {
        let (mut checker, mut reports) = checked(SOURCE, false);
        let (&id, op) = checker
            .emissions
            .iter()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let port = Port::Emission(op.targets[0].id);
        checker
            .emissions
            .get_mut(&id)
            .unwrap()
            .composed
            .as_mut()
            .unwrap()
            .shared_primary = Some(primary);
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .operation_effects_limited(&reports, Span::default(), 1, 8, 0)
                .unwrap_err()
                .message
                .contains("identity")
        );
        let mut effects = Effects::new();
        let mut parts = 8;
        assert!(
            checker
                .record_emission_effect(
                    0,
                    (id, Some(0)),
                    &mut effects,
                    1,
                    &mut parts,
                    Span::default()
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(effects.is_empty());
        assert_eq!(parts, 8);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
