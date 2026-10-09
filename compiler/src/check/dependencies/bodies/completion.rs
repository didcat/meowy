use crate::{check::dependencies::ScalarKind, hir};

pub(crate) const MAX_BODY_RESULTS: usize = super::super::sequences::MAX_ITEMS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    Never,
    Scalar(ScalarKind),
    SharedScalar(ScalarKind),
    Record { fields: usize },
    List { capacity: usize },
    Reference(hir::ReferenceMode),
    Union { members: usize },
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Completion {
    pub(crate) normal: bool,
    pub(crate) result: Shape,
}

impl Completion {
    pub(crate) fn of(ty: &hir::Type) -> Self {
        use hir::Type;
        let result = match ty {
            Type::Never => Shape::Never,
            Type::Null => Shape::Scalar(ScalarKind::Null),
            Type::Bool => Shape::Scalar(ScalarKind::Bool),
            Type::Int { bits, signed } if matches!(bits, 8 | 16 | 32 | 64) => {
                Shape::Scalar(ScalarKind::Int {
                    bits: *bits,
                    signed: *signed,
                })
            }
            Type::Float { bits } if matches!(bits, 32 | 64) => {
                Shape::Scalar(ScalarKind::Float { bits: *bits })
            }
            Type::String => Shape::Scalar(ScalarKind::String),
            Type::Record { fields, .. } if fields.len() <= MAX_BODY_RESULTS => Shape::Record {
                fields: fields.len(),
            },
            Type::List { capacity, .. } if *capacity <= crate::list::MAX_CAPACITY => Shape::List {
                capacity: *capacity,
            },
            Type::Reference(ty) => ScalarKind::of(ty).map_or(
                Shape::Reference(hir::ReferenceMode::Shared),
                Shape::SharedScalar,
            ),
            Type::Exclusive(_) => Shape::Reference(hir::ReferenceMode::Exclusive),
            Type::Union(members) if (2..=MAX_BODY_RESULTS).contains(&members.len()) => {
                Shape::Union {
                    members: members.len(),
                }
            }
            _ => Shape::Other,
        };
        Self {
            normal: *ty != Type::Never,
            result,
        }
    }

    pub(crate) fn valid(self) -> bool {
        if self.normal != (self.result != Shape::Never) {
            return false;
        }
        match self.result {
            Shape::Scalar(ScalarKind::Int { bits, .. })
            | Shape::SharedScalar(ScalarKind::Int { bits, .. }) => matches!(bits, 8 | 16 | 32 | 64),
            Shape::Scalar(ScalarKind::Float { bits })
            | Shape::SharedScalar(ScalarKind::Float { bits }) => matches!(bits, 32 | 64),
            Shape::Record { fields } => fields <= MAX_BODY_RESULTS,
            Shape::List { capacity } => capacity <= crate::list::MAX_CAPACITY,
            Shape::Union { members } => (2..=MAX_BODY_RESULTS).contains(&members),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests;
