use super::*;
use crate::flow::TRUE;

impl State {
    pub(in super::super) fn restrict(
        &self,
        guard: Guard,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<Self> {
        if guard == TRUE {
            return self.snapshot(flow, span, limit);
        }
        self.validate(flow, span)?;
        if limit == 0 || !flow.spend(2) {
            return Err(budget(span));
        }
        if guard >> 1 >= flow.nodes.len() {
            return Err(invalid(span));
        }
        let reach = flow.and(self.reach, guard);
        if flow.exceeded() {
            return Err(budget(span));
        }
        self.copy(reach, flow, span, limit)
    }

    pub(in super::super) fn join(
        &self,
        other: &Self,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<Self> {
        self.validate(flow, span)?;
        if limit == 0 || !flow.spend(3) {
            return Err(budget(span));
        }
        if self.owner != other.owner {
            return Err(invalid(span));
        }
        if self.reach == FALSE {
            return other.restrict(TRUE, flow, span, limit);
        }
        other.validate(flow, span)?;
        if other.reach == FALSE {
            return self.copy(self.reach, flow, span, limit);
        }
        let reach = flow.or(self.reach, other.reach);
        if flow.exceeded() {
            return Err(budget(span));
        }
        let mut state = Self::new(self.owner, reach, flow, span)?;
        for &local in self.cells.keys().chain(other.cells.keys()) {
            if !flow.spend(
                self.cells.len().checked_ilog2().unwrap_or(0) as usize
                    + other.cells.len().checked_ilog2().unwrap_or(0) as usize
                    + state.cells.len().checked_ilog2().unwrap_or(0) as usize
                    + 4,
            ) {
                return Err(budget(span));
            }
            if state.cells.contains_key(&local) {
                continue;
            }
            let cell = Cell::join(self.get(local), other.get(local), flow, span)?;
            state.put(local, &cell, flow, span, limit)?;
        }
        Ok(state)
    }
}

impl Cell {
    pub(super) fn join(
        left: Option<&Self>,
        right: Option<&Self>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Self> {
        if !flow.spend(
            left.map_or(0, Self::weight)
                .saturating_add(right.map_or(0, Self::weight))
                .saturating_add(4),
        ) {
            return Err(budget(span));
        }
        let control =
            left.is_some_and(|cell| cell.control) || right.is_some_and(|cell| cell.control);
        let data = match (left.map(|cell| &cell.data), right.map(|cell| &cell.data)) {
            (None, _) | (_, None) | (Some(Data::Unavailable), _) | (_, Some(Data::Unavailable)) => {
                Data::Unavailable
            }
            (Some(Data::Uninitialized), Some(Data::Uninitialized)) => Data::Uninitialized,
            (
                Some(Data::Initialized {
                    version: a,
                    domain: x,
                    origin: p,
                }),
                Some(Data::Initialized {
                    version: b,
                    domain: y,
                    origin: q,
                }),
            ) => {
                let domain = match (x, y) {
                    (Some(x), Some(y)) => Some(x.join(y, flow, span)?),
                    _ => None,
                };
                Data::Initialized {
                    version: if a == b { *a } else { None },
                    origin: if domain.is_some() && p == q { *p } else { None },
                    domain,
                }
            }
            _ => Data::Partial,
        };
        Ok(Self { data, control })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
