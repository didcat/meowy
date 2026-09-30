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

#[test]
pub(crate) fn indirect_effects_keep_indexed_pointee_owners_aliases_and_control() {
    for source in [
        "x:={->n:=1};p:&!(x.n);q:&!*p;*q=2",
        "x:=[[1]];p:&!(x[1][1]);*p=2",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let (&id, op) = checker.stores.first_key_value().unwrap();
        let Effect::Indirect { origins, .. } = &reports
            .effects
            .get(&id)
            .unwrap_or_else(|| panic!("{source}"))
            .1
        else {
            panic!()
        };
        assert_eq!(origins, &op.origins);
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            BTreeSet::from([checker.operations.first_key_value().unwrap().1.storage])
        );
    }
    let (_, reports) = checked(
        "f:(flag<boolean>)'out{|flag|{'out->n:=1;p:&!n;*p=2};|!flag|{'out->n:=1;p:&!n;*p=3}}",
        false,
    );
    let origins: Vec<_> = reports
        .effects
        .values()
        .filter_map(|(_, effect)| {
            if let Effect::Indirect { origins, .. } = effect {
                Some(origins)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(origins.len(), 2);
    assert_eq!(origins[0], origins[1]);
    let (checker, reports) = checked(
        "flag:false;x:=1;p:&!x;|flag|*p=2;f:(){y:=3;q:&!y;*q=4}",
        true,
    );
    assert_eq!(checker.stores.len(), 2);
    for (id, op) in &checker.stores {
        let (owner, Effect::Indirect { control, .. }) = &reports.effects[id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(*control, op.control);
        assert_eq!(*control, *owner == 0);
    }
}

#[test]
pub(crate) fn indirect_effects_preserve_target_errors_stopped_rhs_and_conditional_targets() {
    assert_eq!(
        crate::compile("x:=1;'out{*({'out.leave();->&!x})=2}").unwrap_err()[0].code,
        "E305"
    );
    for (source, present) in [
        ("x:=1;id<&!int32>:(p<&!int32>){->p};*(id(&!x))=2", true),
        ("x:=1;p:&!x;'out{*p={'out.leave();->2}}", false),
        (
            "flag:=false;x:=1;'out{*({|flag|'out.leave();->&!x})=2}",
            true,
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert_eq!(checker.stores.len(), 1);
        let id = checker.stores.first_key_value().unwrap().0;
        assert_eq!(reports.effects.contains_key(id), present, "{source}");
    }
}

#[test]
pub(crate) fn indirect_effects_keep_calls_opaque_after_function_declarations() {
    let source = "id<&!int32>:(p<&!int32>){->p};x:=1;*(id(&!x))=2";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let id = *checker.stores.first_key_value().unwrap().0;
    let Effect::Indirect { origins, .. } = &reports.effects[&id].1 else {
        panic!()
    };
    assert_eq!(origins, &checker.stores[&id].origins);
    assert_eq!(checker.invocations.len(), 1);
    let call = checker.invocations.first_key_value().unwrap().1;
    assert!(
        matches!(reports.effects[&call.point].1, Effect::Call { function, .. } if function == call.function)
    );
}

#[test]
pub(crate) fn indirect_effects_copy_duplicate_origins_once_and_preserve_empty_completeness() {
    let (mut checker, mut reports) = checked("x:=1;p:&!x;*p=2", false);
    let id = *checker.stores.first_key_value().unwrap().0;
    let expected = reports.effects.clone();
    reports
        .entries
        .get_mut(&0)
        .unwrap()
        .1
        .ports
        .extend([Port::Operation(id); 3]);
    assert_eq!(
        checker
            .operation_effects_limited(&reports, Span::default(), expected.len(), 1, 1)
            .unwrap(),
        expected
    );
    for complete in [false, true] {
        checker.stores.get_mut(&id).unwrap().origins = Origins {
            roots: BTreeSet::new(),
            complete,
        };
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), expected.len(), 1, 0)
            .unwrap();
        let Effect::Indirect { origins, .. } = &effects[&id].1 else {
            panic!()
        };
        assert!(origins.roots.is_empty());
        assert_eq!(origins.complete, complete);
    }
    assert_eq!(reports.effects, expected);
}

#[test]
pub(crate) fn indirect_effects_reject_foreign_owners_and_seeded_snapshot_overflow() {
    let (mut checker, reports) = checked("x:=1;p:&!x;*p=2", false);
    let id = *checker.stores.first_key_value().unwrap().0;
    let expected = reports.effects.clone();
    checker.stores.get_mut(&id).unwrap().owner = 1;
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("owner mismatch")
    );
    checker.stores.get_mut(&id).unwrap().owner = 0;
    checker.stores.get_mut(&id).unwrap().origins.roots = (0..MAX_ROOTS).collect();
    let full = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    let Effect::Indirect { origins, .. } = &full[&id].1 else {
        panic!()
    };
    assert_eq!(origins.roots.len(), MAX_ROOTS);
    checker
        .stores
        .get_mut(&id)
        .unwrap()
        .origins
        .roots
        .insert(MAX_ROOTS);
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(origins.roots.len(), MAX_ROOTS);
    assert_eq!(reports.effects, expected);
}

#[test]
pub(crate) fn indirect_effects_preserve_reports_and_marks_on_work_exhaustion() {
    let (mut checker, reports) = checked("flag:false;x:=1;p:&!x;|flag|*p=2;*p=3", true);
    let counts = checker.edge_counts();
    let marks = checker.derived.clone();
    assert!(!marks.is_empty());
    let before = checker.flow.work;
    let expected = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    for room in [0, work / 2, work - 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - room;
        assert!(
            checker
                .operation_effects(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.flow.exceeded());
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
        assert_eq!(checker.derived, marks);
    }
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    assert_eq!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap(),
        expected
    );
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
}
