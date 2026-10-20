use crate::ast::Span;
use crate::check::Checker;

pub(crate) const MAX_CONTEXTS: usize = 256;
pub(crate) const MAX_SCALAR_NODES: usize = 4096;

pub(crate) struct Scalar {
    pub(crate) checker: Box<Checker>,
    pub(crate) nodes: usize,
    pub(crate) bytes: usize,
    pub(crate) unknown: bool,
    pub(crate) controls: Vec<(Span, Span)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fit {
    Yes,
    No,
    Unknown,
}

impl Fit {
    pub(crate) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::No, _) | (_, Self::No) => Self::No,
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            _ => Self::Yes,
        }
    }
}
