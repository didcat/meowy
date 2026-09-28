use super::{tests::checked, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn indirect_effects_keep_pre_rhs_origins_when_rhs_retargets_the_reference() {
    let source = "x:=1;y:=2;p:=&!x;*p={p=&!y;->3};*p=4";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.stores.len(), 2);
    let mut snapshots = Vec::new();
    for (&id, op) in &checker.stores {
        let (owner, effect) = &reports.effects[&id];
        assert_eq!(*owner, op.owner);
        assert_eq!(
            *effect,
            Effect::Indirect {
                target: op.target,
                input: op.input,
                origins: op.origins.clone(),
                control: op.control,
            }
        );
        let Effect::Indirect {
            target,
            input,
            origins,
            ..
        } = effect
        else {
            panic!()
        };
        assert_ne!(target, input);
        assert_eq!(checker.points[*target].parent, Some(id));
        assert_eq!(checker.points[*input].parent, Some(id));
        assert!(origins.complete);
        assert!(!origins.roots.contains(&2));
        snapshots.push(origins.roots.clone());
    }
    assert_eq!(snapshots, [BTreeSet::from([0]), BTreeSet::from([0, 1])]);
    assert_eq!(checker.pointees[&2].roots, BTreeSet::from([0, 1]));
}

#[test]
pub(crate) fn indirect_effects_preserve_incomplete_origins_without_strengthening_them() {
    let source = "x:=1;y:=2;p:=&!x;p={->&!y};*p=3";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let (&id, op) = checker.stores.first_key_value().unwrap();
    let Effect::Indirect { origins, .. } = &reports.effects[&id].1 else {
        panic!()
    };
    assert_eq!(origins, &op.origins);
    assert!(!origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([0]));
}

#[test]
pub(crate) fn indirect_effects_bound_aggregate_origin_copies_before_publication() {
    let (mut checker, reports) = checked("x:=1;y:=2;p:=&!x;*p={p=&!y;->3};*p=4", false);
    let expected = reports.effects.clone();
    let counts = checker.edge_counts();
    for roots in [0, 1, 2] {
        assert!(
            checker
                .operation_effects_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES, roots)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
    }
    assert_eq!(
        checker
            .operation_effects_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES, 3)
            .unwrap(),
        expected
    );
}
