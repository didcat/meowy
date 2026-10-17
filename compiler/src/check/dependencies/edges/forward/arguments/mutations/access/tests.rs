use super::*;
use crate::flow::MAX_PROOF_WORK;
use crate::hir::{Field, FoundationType};

#[test]
pub(crate) fn declared_access_distinguishes_scalar_modes_and_opaque_types() {
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    let kind = ScalarKind::Int {
        bits: 32,
        signed: true,
    };
    for (ty, expected) in [
        (int.clone(), Access::Value),
        (Type::Never, Access::Value),
        (Type::Reference(Box::new(int.clone())), Access::Shared(kind)),
        (Type::Exclusive(Box::new(int)), Access::Exclusive(kind)),
        (
            Type::Reference(Box::new(Type::String)),
            Access::Shared(ScalarKind::String),
        ),
        (Type::Exclusive(Box::new(Type::String)), Access::Unknown),
        (Type::Foundation(FoundationType::Allocator), Access::Unknown),
        (
            Type::Int {
                bits: 7,
                signed: false,
            },
            Access::Unknown,
        ),
        (
            Type::Exclusive(Box::new(Type::Int {
                bits: 7,
                signed: false,
            })),
            Access::Unknown,
        ),
        (
            Type::Reference(Box::new(Type::Reference(Box::new(Type::Bool)))),
            Access::Unknown,
        ),
    ] {
        assert_eq!(
            classify(&ty, &mut Flow::new(), Span::default()).unwrap(),
            expected
        );
    }
}

#[test]
pub(crate) fn declared_access_checks_record_primary_fields_lists_and_union_alternatives() {
    for hidden in [
        Type::Reference(Box::new(Type::Bool)),
        Type::Exclusive(Box::new(Type::Bool)),
        Type::Foundation(FoundationType::Allocator),
    ] {
        for ty in [
            Type::Record {
                primary: Box::new(hidden.clone()),
                fields: vec![],
            },
            Type::Record {
                primary: Box::new(Type::Null),
                fields: vec![Field {
                    name: "p".into(),
                    ty: hidden.clone(),
                    mutable: false,
                }],
            },
            Type::List {
                element: Box::new(hidden.clone()),
                capacity: 0,
            },
            Type::Union(vec![Type::Null, hidden]),
        ] {
            assert_eq!(
                classify(&ty, &mut Flow::new(), Span::default()).unwrap(),
                Access::Unknown
            );
        }
    }
    let plain = Type::Record {
        primary: Box::new(Type::Bool),
        fields: vec![Field {
            name: "n".into(),
            ty: Type::List {
                element: Box::new(Type::Union(vec![Type::Null, Type::Bool])),
                capacity: 2,
            },
            mutable: true,
        }],
    };
    assert_eq!(
        classify(&plain, &mut Flow::new(), Span::default()).unwrap(),
        Access::Value
    );
    assert_eq!(
        classify(
            &Type::Reference(Box::new(plain)),
            &mut Flow::new(),
            Span::default()
        )
        .unwrap(),
        Access::Unknown
    );
}

#[test]
pub(crate) fn declared_access_bounds_complete_type_traversal_and_exact_work() {
    let ty = Type::Union(vec![Type::Null, Type::Reference(Box::new(Type::Bool))]);
    let mut flow = Flow::new();
    classify(&ty, &mut flow, Span::default()).unwrap();
    let work = flow.work;
    for extra in [0, 1] {
        let mut flow = Flow::new();
        flow.work = MAX_PROOF_WORK - work + extra;
        assert_eq!(
            classify(&ty, &mut flow, Span::default()).is_ok(),
            extra == 0
        );
    }
    let huge = Type::Reference(Box::new(Type::Union(vec![
        Type::Bool;
        crate::borrow_value::MAX_PARTS
            + 1
    ])));
    assert!(classify(&huge, &mut Flow::new(), Span::default()).is_err());
}
