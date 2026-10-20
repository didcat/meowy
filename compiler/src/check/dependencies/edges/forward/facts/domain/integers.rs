use super::{budget, invalid};
use crate::{ast::Span, check::Result, flow::Flow};

pub(super) const LIMIT: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) enum Integers {
    Set(Vec<i128>),
    Interval(i128, i128),
}

impl Integers {
    pub(super) fn set(values: &[i128], flow: &mut Flow, span: Span) -> Result<Option<Self>> {
        if values.len() <= 1 {
            if !flow.spend(values.len() + 1) {
                return Err(budget(span));
            }
            return Ok(values.first().map(|value| Self::Set(vec![*value])));
        }
        if !flow.spend(values.len().saturating_mul(LIMIT + 4).saturating_add(1)) {
            return Err(budget(span));
        }
        let Some(&first) = values.first() else {
            return Ok(None);
        };
        let (low, high) = values.iter().fold((first, first), |(low, high), value| {
            (low.min(*value), high.max(*value))
        });
        let mut set = Vec::with_capacity(values.len().min(LIMIT));
        for value in values {
            if let Err(index) = set.binary_search(value) {
                if set.len() == LIMIT {
                    return Self::interval(low, high, flow, span);
                }
                set.insert(index, *value);
            }
        }
        Ok(Some(Self::Set(set)))
    }

    pub(super) fn interval(
        low: i128,
        high: i128,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<Self>> {
        if !flow.spend(1) {
            return Err(budget(span));
        }
        if low > high {
            return Ok(None);
        }
        if let Some(width) = high.checked_sub(low).filter(|width| *width < LIMIT as i128) {
            if !flow.spend(width as usize + 1) {
                return Err(budget(span));
            }
            return Ok(Some(Self::Set(
                (0..=width).map(|offset| low + offset).collect(),
            )));
        }
        Ok(Some(Self::Interval(low, high)))
    }

    pub(super) fn valid(&self) -> bool {
        match self {
            Self::Set(values) => {
                !values.is_empty()
                    && values.len() <= LIMIT
                    && values.windows(2).all(|pair| pair[0] < pair[1])
            }
            Self::Interval(low, high) => {
                low <= high
                    && high
                        .checked_sub(*low)
                        .is_none_or(|width| width >= LIMIT as i128)
            }
        }
    }

    pub(super) fn weight(&self) -> usize {
        match self {
            Self::Set(values) => values.len() + 1,
            Self::Interval(..) => 3,
        }
    }

    pub(self) fn bounds(&self) -> (i128, i128) {
        match self {
            Self::Set(values) => (values[0], values[values.len() - 1]),
            Self::Interval(low, high) => (*low, *high),
        }
    }

    pub(super) fn join(&self, other: &Self, flow: &mut Flow, span: Span) -> Result<Self> {
        if !flow.spend(
            self.weight()
                .saturating_add(other.weight())
                .saturating_add(1),
        ) {
            return Err(budget(span));
        }
        if !self.valid() || !other.valid() {
            return Err(invalid(span));
        }
        if let (Self::Set(left), Self::Set(right)) = (self, other) {
            let values = left.iter().chain(right).copied().collect::<Vec<_>>();
            return Ok(Self::set(&values, flow, span)?.expect("nonempty union"));
        }
        let (left, right) = self.bounds();
        let (low, high) = other.bounds();
        Ok(Self::interval(left.min(low), right.max(high), flow, span)?.expect("nonempty hull"))
    }
}

#[cfg(test)]
mod tests;
