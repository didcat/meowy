use super::super::ScalarKind;
use crate::hir::{ReferenceMode, Type};

pub(crate) const MAX_COUNT: usize = super::super::sequences::MAX_ITEMS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    Never,
    Scalar(ScalarKind),
    SharedScalar(ScalarKind),
    Record { fields: usize },
    List { capacity: usize },
    Reference(ReferenceMode),
    Union { members: usize },
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Types {
    pub(crate) inputs: [Class; 2],
    pub(crate) result: Class,
}

impl Class {
    pub(super) fn of(ty: &Type) -> Self {
        match ty {
            Type::Never => Self::Never,
            Type::Null => Self::Scalar(ScalarKind::Null),
            Type::Bool => Self::Scalar(ScalarKind::Bool),
            Type::Int { bits, signed } => Self::Scalar(ScalarKind::Int {
                bits: *bits,
                signed: *signed,
            }),
            Type::Float { bits } => Self::Scalar(ScalarKind::Float { bits: *bits }),
            Type::String => Self::Scalar(ScalarKind::String),
            Type::Record { fields, .. } if fields.len() <= MAX_COUNT => Self::Record {
                fields: fields.len(),
            },
            Type::List { capacity, .. } if *capacity <= crate::list::MAX_CAPACITY => Self::List {
                capacity: *capacity,
            },
            Type::Reference(ty) => ScalarKind::of(ty)
                .map_or(Self::Reference(ReferenceMode::Shared), Self::SharedScalar),
            Type::Exclusive(_) => Self::Reference(ReferenceMode::Exclusive),
            Type::Union(members) if (2..=MAX_COUNT).contains(&members.len()) => Self::Union {
                members: members.len(),
            },
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests;
