use super::*;
use crate::hir::Type;

#[test]
pub(crate) fn scalar_domains_separate_bottom_null_boolean_subsets_and_nullable_values() {
    let mut flow = Flow::default();
    let span = Span::default();
    let no = Domain::boolean(false);
    let yes = Domain::boolean(true);
    let both = no.join(&yes, &mut flow, span).unwrap();
    assert_eq!(
        both,
        Domain {
            values: Some(Values::Bool(3)),
            null: false
        }
    );
    assert_ne!(Domain::BOTTOM, Domain::NULL);
    assert_eq!(Domain::BOTTOM.join(&yes, &mut flow, span).unwrap(), yes);
    assert_eq!(
        Domain::BOTTOM.join(&Domain::NULL, &mut flow, span).unwrap(),
        Domain::NULL
    );
    let nullable = both.join(&Domain::NULL, &mut flow, span).unwrap();
    assert!(nullable.null);
    assert_eq!(nullable.values, both.values);
    for ty in [
        Type::Union(vec![Type::Null, Type::Bool]),
        Type::Union(vec![Type::Bool, Type::Null]),
    ] {
        assert_eq!(
            Domain::declared(&ty, &mut flow, span).unwrap(),
            Some(nullable.clone())
        );
    }
    assert_eq!(
        Domain::declared(&Type::Never, &mut flow, span).unwrap(),
        Some(Domain::BOTTOM)
    );
    assert_eq!(
        Domain::declared(&Type::Null, &mut flow, span).unwrap(),
        Some(Domain::NULL)
    );
}

#[test]
pub(crate) fn scalar_declared_domains_keep_full_integer_widths_and_nullable_ranges() {
    let mut flow = Flow::default();
    let span = Span::default();
    for bits in [8, 16, 32, 64] {
        for signed in [false, true] {
            let ty = Type::Int { bits, signed };
            let low = if signed { -(1i128 << (bits - 1)) } else { 0 };
            let high = (1i128 << (bits - u32::from(signed))) - 1;
            let expected = Domain {
                values: Some(Values::Int(Integers::Interval(low, high))),
                null: false,
            };
            assert_eq!(
                Domain::declared(&ty, &mut flow, span).unwrap(),
                Some(expected.clone())
            );
            assert_ne!(Domain::integer(0, &mut flow, span).unwrap(), expected);
            let mut nullable = expected;
            nullable.null = true;
            assert_eq!(
                Domain::declared(&Type::Union(vec![ty, Type::Null]), &mut flow, span).unwrap(),
                Some(nullable)
            );
        }
    }
}

#[test]
pub(crate) fn scalar_domains_keep_unsupported_shapes_distinct_from_empty_domains() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    for ty in [
        Type::Float { bits: 32 },
        Type::String,
        Type::Reference(Box::new(int.clone())),
        Type::Exclusive(Box::new(int.clone())),
        Type::Foundation(crate::hir::FoundationType::Allocator),
        Type::Record {
            primary: Box::new(int.clone()),
            fields: vec![],
        },
        Type::List {
            element: Box::new(int.clone()),
            capacity: 1,
        },
        Type::Int {
            bits: 0,
            signed: true,
        },
        Type::Int {
            bits: 128,
            signed: false,
        },
        Type::Union(vec![]),
        Type::Union(vec![Type::Null, Type::Null]),
        Type::Union(vec![Type::Null, Type::Never]),
        Type::Union(vec![Type::Bool, int.clone()]),
        Type::Union(vec![Type::Null, Type::Bool, int]),
    ] {
        assert_eq!(
            Domain::declared(&ty, &mut Flow::default(), Span::default()).unwrap(),
            None,
            "{ty:?}"
        );
    }
}

#[test]
pub(crate) fn scalar_domain_joins_reject_malformed_or_mixed_nonnull_payloads() {
    let mut flow = Flow::default();
    let span = Span::default();
    for value in [
        Domain {
            values: Some(Values::Bool(0)),
            null: true,
        },
        Domain {
            values: Some(Values::Bool(4)),
            null: false,
        },
        Domain {
            values: Some(Values::Int(Integers::Set(vec![]))),
            null: true,
        },
    ] {
        assert!(
            value
                .join(&Domain::NULL, &mut flow, span)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    let number = Domain::integer(1, &mut flow, span).unwrap();
    assert!(
        number
            .join(&Domain::boolean(true), &mut flow, span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    let nullable = number.join(&Domain::NULL, &mut flow, span).unwrap();
    assert_eq!(nullable.values, number.values);
    assert!(nullable.null);
}

#[test]
pub(crate) fn scalar_declared_domains_bound_exact_nullable_construction_work() {
    let ty = Type::Union(vec![
        Type::Null,
        Type::Int {
            bits: 64,
            signed: false,
        },
    ]);
    let mut flow = Flow::default();
    let span = Span::default();
    let expected = Domain::declared(&ty, &mut flow, span).unwrap();
    let cost = flow.work;
    for extra in [0, 1] {
        flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        flow.full = false;
        let result = Domain::declared(&ty, &mut flow, span);
        assert_eq!(result.is_ok(), extra == 0);
        if let Ok(value) = result {
            assert_eq!(value, expected);
        }
    }
}
