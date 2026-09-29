use super::super::ScalarKind;
use crate::hir::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    Never,
    Scalar(ScalarKind),
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
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests;
