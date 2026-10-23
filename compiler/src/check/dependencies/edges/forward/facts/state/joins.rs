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
        let room = limit
            .min(MAX_EDGES)
            .checked_sub(1)
            .ok_or_else(|| budget(span))?;
        let (cells, used) = self.cells.joined(&other.cells, flow, span, room)?;
        let (slots, extra) = if self.slots.is_empty() && other.slots.is_empty() {
            (tree::Tree::new(self.owner), 0)
        } else {
            self.slots.joined(
                &other.slots,
                flow,
                span,
                room.checked_sub(used).ok_or_else(|| budget(span))?,
            )?
        };
        let bottom = reach == FALSE || cells.bottom() || slots.bottom();
        let parts = if bottom {
            1
        } else {
            1 + cells.weight() + slots.weight()
        };
        if parts > MAX_EDGES {
            return Err(budget(span));
        }
        Ok(Self {
            owner: self.owner,
            reach: if bottom { FALSE } else { reach },
            parts,
            cells: if bottom {
                tree::Tree::new(self.owner)
            } else {
                cells
            },
            slots: if bottom {
                tree::Tree::new(self.owner)
            } else {
                slots
            },
            used: used + extra + 1,
        })
    }
}

impl Cell {
    pub(super) fn join(
        left: Option<&Self>,
        right: Option<&Self>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Self> {
        if !flow.spend(4) {
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
                if !flow.spend(left.unwrap().weight() + right.unwrap().weight()) {
                    return Err(budget(span));
                }
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
