use super::*;
use crate::check::dependencies::edges::forward::{
    consumers::tests::checked, results::inputs::graph::Visit,
};

#[test]
pub(crate) fn local_field_sources_retain_reads_initializers_blocks_and_owners() {
    for source in [
        "r:{->n:1};n:r.n;s:{->n};f<null>:(){r:{->n:2};n:r.n;s:{->n}}",
        "r:{->n:1};n<int32>:((r.n~<int32>));s:{copy<int32>:((n~<int32>));inner:{->n<int32>:((copy~<int32>))}};f<null>:(){r:{->n:2};n<int32>:r.n;s:{copy:n;inner:{->n<int32>:((copy~<int32>))}}}",
    ] {
        let (mut checker, reports) = checked(source);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut owners = BTreeSet::new();
        let mut crossed = false;
        let mut ctx = Lookup::new(&reports, MAX_EDGES);
        for (&key, &(owner, direct)) in &reports.direct_sources {
            let input = reports.candidate_inputs[&key].1;
            let mut current = input.point;
            let mut reads = 0;
            let mut hops = 0;
            loop {
                current = if let Some(group) = checker.group_inputs.get(&current) {
                    group.input
                } else if let Some(op) = checker.coercions.get(&current) {
                    op.input
                } else if let Some(op) = checker.typed_ops.get(&current) {
                    op.input
                } else if let Some(op) = checker.narrowings.get(&current) {
                    op.input
                } else if let Some(op) = checker.local_reads.get(&current) {
                    reads += 1;
                    let root = reports.initializers[&op.local].input.unwrap();
                    assert!(reports.eligible.contains(&op.local));
                    assert_ne!(checker.points[root].parent, Some(current));
                    crossed |= checker.points[root].block != checker.points[current].block;
                    assert_eq!(
                        op.edges,
                        [
                            Edge::new(Port::Entry(current), Port::Operation(current), Route::Next),
                            Edge::new(Port::Operation(current), Port::Normal(current), Route::Next),
                        ]
                    );
                    assert_eq!(
                        checker
                            .read_initializer_input(&reports, current, owner, Span::default())
                            .unwrap(),
                        Some(root)
                    );
                    root
                } else {
                    break;
                };
                hops += 1;
            }
            if reads == 0 {
                continue;
            }
            let field = direct.source.expect("immutable local field source");
            assert_eq!(current, field.field);
            assert_eq!(input.point, direct.point);
            assert_ne!(input.point, field.field);
            assert_eq!(input.source, None);
            assert_eq!(
                checker
                    .field_narrowing_source(&mut ctx, input.point, owner, Span::default(), hops)
                    .unwrap(),
                Some(field)
            );
            assert_eq!(
                reports.field_results[&Port::Normal(field.field)],
                (owner, field.slot)
            );
            assert!(
                reports
                    .candidate_walk
                    .visits
                    .contains(&Visit::Value(key, input))
            );
            assert!(
                reports
                    .expanded_walk
                    .visits
                    .contains(&Visit::Field(key, input, field))
            );
            assert!(
                !reports
                    .expanded_walk
                    .visits
                    .contains(&Visit::Value(key, input))
            );
            assert_eq!(
                checker
                    .grouped_consumer(&reports, input.point, owner, Span::default())
                    .unwrap(),
                None
            );
            owners.insert(owner);
        }
        assert_eq!(owners, BTreeSet::from([0, 1]), "{source}");
        assert!(crossed);
        assert_eq!(
            checker.direct_sources(&reports, Span::default()).unwrap().0,
            reports.direct_sources
        );
        assert_eq!(
            checker
                .candidate_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.candidate_walk
        );
        assert_eq!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.expanded_walk
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn local_field_sources_retain_distinct_reads_of_one_initializer_field() {
    let (mut checker, reports) = checked("r:{->n:1};n:r.n;copy:n;s:{->n;->other:((copy~<int32>))}");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut points = BTreeSet::new();
    let mut fields = BTreeSet::new();
    let mut slots = BTreeSet::new();
    for (&key, &(_, direct)) in &reports.direct_sources {
        let Some(field) = direct.source else {
            continue;
        };
        let input = reports.candidate_inputs[&key].1;
        points.insert(direct.point);
        fields.insert(field.field);
        slots.insert(field.slot);
        assert!(
            reports
                .candidate_walk
                .visits
                .contains(&Visit::Value(key, input))
        );
        assert!(
            reports
                .expanded_walk
                .visits
                .contains(&Visit::Field(key, input, field))
        );
    }
    assert_eq!((points.len(), fields.len(), slots.len()), (2, 1, 1));
    assert_eq!(
        checker
            .direct_walk_report(&reports, Span::default())
            .unwrap()
            .0,
        reports.expanded_walk
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
