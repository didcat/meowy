use super::completion::Shape;
use crate::{check::dependencies::ScalarKind, hir::Type};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Scalar {
    pub(crate) kind: ScalarKind,
    pub(crate) nullable: bool,
}

impl Scalar {
    pub(crate) fn of(ty: &Type) -> Option<Self> {
        let (ty, nullable) = match ty {
            Type::Union(types) if types.len() == 2 => match (&types[0], &types[1]) {
                (Type::Null, ty) | (ty, Type::Null) => (ty, true),
                _ => return None,
            },
            ty => (ty, false),
        };
        let kind = match ty {
            Type::Null if !nullable => ScalarKind::Null,
            Type::Bool => ScalarKind::Bool,
            Type::Int { bits, signed } if matches!(bits, 8 | 16 | 32 | 64) => ScalarKind::Int {
                bits: *bits,
                signed: *signed,
            },
            _ => return None,
        };
        Some(Self { kind, nullable })
    }

    pub(crate) fn shape(self) -> Option<Shape> {
        match self.kind {
            ScalarKind::Null if !self.nullable => {}
            ScalarKind::Bool => {}
            ScalarKind::Int {
                bits: 8 | 16 | 32 | 64,
                ..
            } => {}
            _ => return None,
        }
        Some(if self.nullable {
            Shape::Union { members: 2 }
        } else {
            Shape::Scalar(self.kind)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Completion, Layout, tests::check};
    use super::*;

    #[test]
    pub(crate) fn scalar_body_types_retain_exact_nullable_alternatives_and_integer_widths() {
        let types = std::iter::once(Type::Bool).chain(
            [8, 16, 32, 64]
                .into_iter()
                .flat_map(|bits| [false, true].map(|signed| Type::Int { bits, signed })),
        );
        for ty in types {
            let plain = Scalar::of(&ty).unwrap();
            assert!(!plain.nullable);
            assert_eq!(plain.shape(), Some(Completion::of(&ty).result));
            for types in [vec![ty.clone(), Type::Null], vec![Type::Null, ty.clone()]] {
                let ty = Type::Union(types);
                let scalar = Scalar::of(&ty).unwrap();
                assert_eq!(scalar.kind, plain.kind);
                assert!(scalar.nullable);
                assert_eq!(scalar.shape(), Some(Completion::of(&ty).result));
            }
        }
        assert_eq!(
            Scalar::of(&Type::Null),
            Some(Scalar {
                kind: ScalarKind::Null,
                nullable: false,
            })
        );
    }

    #[test]
    pub(crate) fn scalar_body_types_exclude_opaque_nested_and_heterogeneous_unions() {
        for ty in [
            Type::Never,
            Type::String,
            Type::Float { bits: 32 },
            Type::Int {
                bits: 7,
                signed: true,
            },
            Type::Reference(Box::new(Type::Bool)),
            Type::Union(vec![Type::Bool]),
            Type::Union(vec![Type::Bool, Type::Bool]),
            Type::Union(vec![Type::Null, Type::Null]),
            Type::Union(vec![Type::Null, Type::Bool, Type::String]),
            Type::Union(vec![Type::Null, Type::Union(vec![Type::Null, Type::Bool])]),
        ] {
            assert_eq!(Scalar::of(&ty), None, "{ty:?}");
            assert_eq!(Scalar::of(&Type::Union(vec![ty, Type::Null])), None);
        }
        for scalar in [
            Scalar {
                kind: ScalarKind::Null,
                nullable: true,
            },
            Scalar {
                kind: ScalarKind::String,
                nullable: false,
            },
            Scalar {
                kind: ScalarKind::Int {
                    bits: 7,
                    signed: false,
                },
                nullable: true,
            },
        ] {
            assert_eq!(scalar.shape(), None);
        }
    }

    #[test]
    pub(crate) fn scalar_body_replay_rejects_same_shape_nullable_type_drift_atomically() {
        let (mut checker, mut block) = check("flag:=true;|flag|->7");
        let body = &checker.bodies[&block.id];
        let span = body.span;
        assert!(body.scalar.unwrap().nullable);
        assert_eq!(body.layout, Layout::Unknown);
        let saved = format!("{:?}{:?}", checker.bodies, checker.endpoints);
        let counts = (checker.body_facts, checker.body_slots, checker.body_names);
        checker.track_body(&block, span).unwrap();
        checker.block_endpoints(&block, span).unwrap();
        for ty in [
            Type::Bool,
            Type::String,
            Type::Int {
                bits: 8,
                signed: false,
            },
        ] {
            block.ty = Type::Union(vec![ty, Type::Null]);
            assert_eq!(
                Completion::of(&block.ty),
                checker.bodies[&block.id].completion
            );
            assert!(
                checker
                    .track_body(&block, span)
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert!(
                checker
                    .block_endpoints(&block, span)
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(
                format!("{:?}{:?}", checker.bodies, checker.endpoints),
                saved
            );
            assert_eq!(
                (checker.body_facts, checker.body_slots, checker.body_names),
                counts
            );
        }
    }
}
