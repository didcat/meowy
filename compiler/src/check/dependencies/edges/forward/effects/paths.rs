use super::{tests::checked, *};

#[test]
pub(crate) fn path_effects_keep_ordered_fields_and_exact_dynamic_index_metadata() {
    let source = "r:={->rows:=[{->xs:=[1,2]}]};i:=1;r.rows[{i=1;->i}].xs[{i=2;->i}]={i=1;->3}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.paths.len(), 1);
    let (&id, op) = checker.paths.first_key_value().unwrap();
    let (owner, effect) = &reports.effects[&id];
    assert_eq!(*owner, op.owner);
    assert_eq!(
        *effect,
        Effect::Path {
            local: op.local,
            storage: op.storage,
            steps: op.steps.clone(),
            input: op.input,
            control: op.control,
        }
    );
    let Effect::Path { steps, .. } = effect else {
        panic!()
    };
    assert_eq!(steps[0], PathStep::Field(0));
    assert_eq!(steps[2], PathStep::Field(0));
    for (step, size, text) in [(1, 1, "{i=1;->i}"), (3, 2, "{i=2;->i}")] {
        let PathStep::Index {
            point,
            capacity,
            span,
        } = steps[step]
        else {
            panic!()
        };
        assert_eq!(capacity, size);
        assert_ne!(point, op.input);
        let root = &checker.points[point];
        assert_eq!(&source[root.span.start..root.span.end], text);
        assert!(span.start < span.end);
    }
}

#[test]
pub(crate) fn path_effects_bound_copied_steps_across_distinct_operations() {
    let (mut checker, reports) = checked("r:={->child:={->n:=1}};r.child.n=2;r.child.n=3", false);
    assert_eq!(checker.paths.len(), 2);
    let expected = reports.effects.clone();
    let counts = checker.edge_counts();
    for steps in [0, 12, 13] {
        assert!(
            checker
                .operation_effects_limited(&reports, Span::default(), MAX_EDGES, steps, MAX_EDGES)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
    }
    assert_eq!(
        checker
            .operation_effects_limited(&reports, Span::default(), MAX_EDGES, 14, MAX_EDGES)
            .unwrap(),
        expected
    );
}

#[test]
pub(crate) fn path_effects_preserve_alias_storage_control_and_independent_owners() {
    let source = "flag:=false;r:'out{|flag|{'out->xs:=[1];xs[1]=2};|!flag|{'out->xs:=[1];xs[1]=3}}";
    crate::compile(source).unwrap();
    let (_, reports) = checked(source, false);
    let paths: Vec<_> = reports
        .effects
        .values()
        .filter_map(|(_, effect)| {
            if let Effect::Path { local, storage, .. } = effect {
                Some((*local, *storage))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(paths.len(), 2);
    assert_ne!(paths[0].0, paths[1].0);
    assert_eq!(paths[0].1, paths[1].1);
    let (checker, reports) = checked(
        "flag:false;r:{->n:=1};|flag|r.n=2;f:(){s:{->n:=1};s.n=3}",
        true,
    );
    assert_eq!(checker.paths.len(), 2);
    for (id, op) in &checker.paths {
        let (owner, Effect::Path { control, .. }) = &reports.effects[id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(*control, op.control);
        assert_eq!(*control, *owner == 0);
    }
}

#[test]
pub(crate) fn path_effects_exclude_stopped_address_and_rhs_operations() {
    for source in [
        "a:=[[1]];'out{a[{'out.leave();->1}][1]=2}",
        "a:=[[1]];'out{a[1][{'out.leave();->1}]=2}",
        "a:=[[1]];'out{a[1][1]={'out.leave();->2}}",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert_eq!(checker.paths.len(), 1);
        for id in checker.paths.keys() {
            assert!(!reports.effects.contains_key(id), "{source}");
        }
    }
}

#[test]
pub(crate) fn path_effects_charge_duplicate_ports_only_once_for_copied_storage() {
    let (mut checker, mut reports) = checked("r:={->child:={->n:=1}};r.child.n=2", false);
    let id = *checker.paths.first_key_value().unwrap().0;
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
            .operation_effects_limited(&reports, Span::default(), expected.len(), 12, MAX_EDGES)
            .unwrap(),
        expected
    );
}

#[test]
pub(crate) fn path_effects_reject_invalid_owners_empty_paths_and_seeded_storage_overflow() {
    let (mut checker, reports) = checked("r:{->n:=1};r.n=2", false);
    let id = *checker.paths.first_key_value().unwrap().0;
    let expected = reports.effects.clone();
    checker.paths.get_mut(&id).unwrap().owner = 1;
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("owner mismatch")
    );
    checker.paths.get_mut(&id).unwrap().owner = 0;
    checker.paths.get_mut(&id).unwrap().steps.clear();
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity mismatch")
    );
    checker.paths.get_mut(&id).unwrap().steps =
        vec![PathStep::Field(0); crate::list::MAX_WRITE_PATH];
    let full = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    let Effect::Path { steps, .. } = &full[&id].1 else {
        panic!()
    };
    assert_eq!(steps.len(), crate::list::MAX_WRITE_PATH);
    checker
        .paths
        .get_mut(&id)
        .unwrap()
        .steps
        .push(PathStep::Field(0));
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(reports.effects, expected);
    assert_eq!(steps.len(), crate::list::MAX_WRITE_PATH);
}

#[test]
pub(crate) fn path_effects_preserve_reports_and_marks_on_early_mid_and_late_work_failure() {
    let (mut checker, reports) = checked(
        "flag:false;r:={->child:={->n:=1}};|flag|r.child.n=2;r.child.n=3",
        true,
    );
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
