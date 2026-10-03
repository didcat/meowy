use super::super::super::tests::checked;
use super::*;
use crate::hir::{Field, FoundationType};

#[test]
pub(crate) fn local_eligibility_rejects_truncated_and_misidentified_local_tables() {
    for fault in 0..4 {
        let (mut checker, mut program) = checked("n:1;unread:2");
        let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
        assert!(checker.local_reads.is_empty());
        match fault {
            0 => reports.locals += 1,
            1 => {
                program.locals.pop();
            }
            2 => {
                program.locals.pop();
                reports.locals = program.locals.len();
            }
            3 => {
                let (id, binding) = checker.proofs.bindings.pop_last().unwrap();
                checker.proofs.bindings.insert(id + 1, binding);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}");
        let edges = checker.edge_counts();
        let error = checker
            .eligible_locals(&program, &reports, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error
                .message
                .contains("local-eligibility identity mismatch")
        );
        assert_eq!(format!("{reports:?}"), before);
        assert_eq!(checker.edge_counts(), edges);
    }
}

#[test]
pub(crate) fn local_eligibility_shares_report_capacity_without_payload_copies() {
    for source in ["", "n:=1", "n:1", "a:1;b:2", "row:{->n:1};copy:({->n:2}).n"] {
        let (mut checker, program) = checked(source);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let used = reports.effects.len()
            + reports.blocks.len()
            + reports.results.len()
            + reports.consumers.len()
            + reports.slot_uses.len()
            + reports.initializers.len();
        let before = format!("{reports:?}");
        for room in 0..=reports.eligible.len() {
            let result =
                checker.eligible_locals_limited(&program, &reports, Span::default(), used + room);
            if room < reports.eligible.len() {
                assert!(result.unwrap_err().message.contains("budget exhausted"));
            } else {
                assert_eq!(result.unwrap(), reports.eligible);
            }
            assert_eq!(format!("{reports:?}"), before);
        }
        if used > 0 {
            assert!(
                checker
                    .eligible_locals_limited(&program, &reports, Span::default(), used - 1)
                    .unwrap_err()
                    .message
                    .contains("budget exhausted")
            );
        }
    }
}

#[test]
pub(crate) fn local_eligibility_bounds_exact_work_and_preserves_reports_on_late_failure() {
    let (mut checker, program) = checked("a:1;b:[2,3];r:{->n:4};copy:r.n");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let start = checker.flow.work;
    assert_eq!(
        checker
            .eligible_locals(&program, &reports, Span::default())
            .unwrap(),
        reports.eligible
    );
    let work = checker.flow.work - start;
    let before = format!("{reports:?}");
    let edges = checker.edge_counts();
    let reads = checker.local_reads.clone();
    let bindings = checker.proofs.bindings.clone();
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.eligible_locals(&program, &reports, Span::default());
        if spare == 0 {
            assert_eq!(result.unwrap(), reports.eligible);
        } else {
            assert!(result.unwrap_err().message.contains("budget exhausted"));
        }
        assert_eq!(format!("{reports:?}"), before);
        assert_eq!(checker.edge_counts(), edges);
        assert_eq!(checker.local_reads, reads);
        assert_eq!(checker.proofs.bindings, bindings);
    }
}

#[test]
pub(crate) fn local_eligibility_type_scratch_bounds_width_without_bounding_depth() {
    let mut checker = Checker::new();
    let mut deep = Type::Bool;
    for _ in 0..=MAX_TYPES {
        deep = Type::List {
            element: Box::new(deep),
            capacity: 1,
        };
    }
    assert!(
        checker
            .eligible_local_type(&deep, Span::default(), 1)
            .unwrap()
    );
    while let Type::List { element, .. } = deep {
        deep = *element;
    }
    for width in [MAX_TYPES, MAX_TYPES + 1] {
        let wide = Type::Union(vec![Type::Bool; width]);
        let result = checker.eligible_local_type(&wide, Span::default(), usize::MAX);
        if width == MAX_TYPES {
            assert!(result.unwrap());
        } else {
            assert!(result.unwrap_err().message.contains("budget exhausted"));
        }
    }
    let record = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![Field {
            name: "n".to_owned(),
            ty: Type::Bool,
            mutable: false,
        }],
    };
    assert!(
        checker
            .eligible_local_type(&record, Span::default(), 2)
            .unwrap()
    );
    assert!(
        checker
            .eligible_local_type(&record, Span::default(), 1)
            .unwrap_err()
            .message
            .contains("budget exhausted")
    );
    assert!(
        checker
            .eligible_local_type(&Type::Bool, Span::default(), 0)
            .is_err()
    );
}

#[test]
pub(crate) fn local_eligibility_rejects_nested_reference_mutation_and_foundation_shapes() {
    let mut checker = Checker::new();
    for leaf in [
        Type::Reference(Box::new(Type::Bool)),
        Type::Exclusive(Box::new(Type::Bool)),
        Type::Foundation(FoundationType::Allocator),
        Type::Foundation(FoundationType::AllocationFailure),
        Type::Foundation(FoundationType::OwnedString),
        Type::Record {
            primary: Box::new(Type::Null),
            fields: vec![Field {
                name: "n".to_owned(),
                ty: Type::Bool,
                mutable: true,
            }],
        },
    ] {
        let ty = Type::Record {
            primary: Box::new(Type::Union(vec![
                Type::Null,
                Type::List {
                    element: Box::new(leaf),
                    capacity: 0,
                },
            ])),
            fields: Vec::new(),
        };
        assert!(
            !checker
                .eligible_local_type(&ty, Span::default(), MAX_TYPES)
                .unwrap()
        );
    }
    let ty = Type::Union(vec![Type::Never, Type::Float { bits: 64 }, Type::String]);
    let start = checker.flow.work;
    assert!(
        checker
            .eligible_local_type(&ty, Span::default(), MAX_TYPES)
            .unwrap()
    );
    let work = checker.flow.work - start;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.eligible_local_type(&ty, Span::default(), MAX_TYPES);
        assert_eq!(result.is_ok(), spare == 0);
    }
}

#[test]
pub(crate) fn local_eligibility_and_initializer_rebuilds_preserve_shared_reports() {
    let (mut checker, program) =
        checked("a:({->n:1}).n;r:{->n:2};copy:r;f<int32>:(p<int32>){local:p;->local}");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    assert!(!reports.eligible.is_empty() && !reports.initializers.is_empty());
    assert_eq!(reports.slot_uses.len(), 1);
    let before = format!("{reports:?}");
    for _ in 0..2 {
        assert_eq!(
            checker
                .eligible_locals(&program, &reports, Span::default())
                .unwrap(),
            reports.eligible
        );
        assert_eq!(
            checker
                .binding_initializers(&program, &reports, Span::default())
                .unwrap(),
            reports.initializers
        );
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
    }
    assert_eq!(format!("{reports:?}"), before);
}
