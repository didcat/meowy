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
    for steps in [0, 2, 3] {
        assert!(
            checker
                .operation_effects_limited(&reports, Span::default(), MAX_EDGES, steps)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
    }
    assert_eq!(
        checker
            .operation_effects_limited(&reports, Span::default(), MAX_EDGES, 4)
            .unwrap(),
        expected
    );
}
