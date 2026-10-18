use super::*;
use crate::flow::Flow;
use integers::Integers;

mod declared;
mod integers;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Values {
    Bool(u8),
    Int(Integers),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Domain {
    pub(super) values: Option<Values>,
    pub(super) null: bool,
}

impl Domain {
    pub(super) const BOTTOM: Self = Self {
        values: None,
        null: false,
    };
    pub(super) const NULL: Self = Self {
        values: None,
        null: true,
    };

    pub(super) fn boolean(value: bool) -> Self {
        Self {
            values: Some(Values::Bool(1 << u8::from(value))),
            null: false,
        }
    }

    pub(super) fn integer(value: i128, flow: &mut Flow, span: Span) -> Result<Self> {
        Ok(Self {
            values: Integers::set(&[value], flow, span)?.map(Values::Int),
            null: false,
        })
    }

    pub(super) fn valid(&self) -> bool {
        match &self.values {
            None => true,
            Some(Values::Bool(values)) => (1..=3).contains(values),
            Some(Values::Int(values)) => values.valid(),
        }
    }

    pub(super) fn weight(&self) -> usize {
        1 + match &self.values {
            None => 0,
            Some(Values::Bool(_)) => 1,
            Some(Values::Int(values)) => values.weight(),
        }
    }

    pub(super) fn join(&self, other: &Self, flow: &mut Flow, span: Span) -> Result<Self> {
        if !flow.spend(
            self.weight()
                .saturating_add(other.weight())
                .saturating_mul(2)
                .saturating_add(1),
        ) {
            return Err(budget(span));
        }
        if !self.valid() || !other.valid() {
            return Err(invalid(span));
        }
        let values = match (&self.values, &other.values) {
            (None, values) | (values, None) => values.clone(),
            (Some(Values::Bool(left)), Some(Values::Bool(right))) => {
                Some(Values::Bool(left | right))
            }
            (Some(Values::Int(left)), Some(Values::Int(right))) => {
                Some(Values::Int(left.join(right, flow, span)?))
            }
            _ => return Err(invalid(span)),
        };
        Ok(Self {
            values,
            null: self.null || other.null,
        })
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-domain budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-domain identity mismatch", span)
}

#[cfg(test)]
mod tests;
