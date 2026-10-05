use super::*;
use crate::check::dependencies::grouped::MAX_GROUPS;
use std::collections::BTreeSet;

mod input;

impl Checker {
    pub(super) fn grouped_consumer(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<PointId>> {
        self.grouped_consumer_limited(reports, input, owner, span, MAX_GROUPS)
    }

    pub(super) fn grouped_consumer_limited(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
        limit: usize,
    ) -> Result<Option<PointId>> {
        self.grouped_source_limited(reports, input, owner, span, limit, false)
    }

    pub(super) fn dispatch_consumer(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<PointId>> {
        self.grouped_source_limited(reports, input, owner, span, MAX_GROUPS, true)
    }

    pub(self) fn grouped_source_limited(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
        limit: usize,
        dispatch: bool,
    ) -> Result<Option<PointId>> {
        let budget = || Diagnostic::unsupported("proof group-consumer budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof group-consumer identity mismatch", span);
        if self.group_inputs.len() > MAX_GROUPS || !self.flow.spend(1) {
            return Err(budget());
        }
        let limit = limit.min(MAX_GROUPS);
        let mut seen = BTreeSet::new();
        let mut current = input;
        loop {
            if !self.flow.spend(
                reports.consumers.len().checked_ilog2().unwrap_or(0) as usize
                    + self.group_inputs.len().checked_ilog2().unwrap_or(0) as usize
                    + self.coercions.len().checked_ilog2().unwrap_or(0) as usize
                    + self.narrowings.len().checked_ilog2().unwrap_or(0) as usize
                    + self.local_reads.len().checked_ilog2().unwrap_or(0) as usize
                    + self.typed_ops.len().checked_ilog2().unwrap_or(0) as usize
                    + 9,
            ) {
                return Err(budget());
            }
            let point = self.points.get(current).ok_or_else(invalid)?;
            if !point.complete
                || point.owner != owner
                || point.span.start > point.span.end
                || !matches!(point.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            {
                return Err(invalid());
            }
            let group = self.group_inputs.get(&current).copied();
            let coercion = self.coercions.contains_key(&current);
            let narrowing = self.narrowings.contains_key(&current);
            let read = self.local_reads.contains_key(&current);
            let typed = self.typed_ops.contains_key(&current);
            if dispatch {
                if !self.flow.spend(
                    self.dispatch_ops.len().checked_ilog2().unwrap_or(0) as usize
                        + self.fields.len().checked_ilog2().unwrap_or(0) as usize
                        + 2,
                ) {
                    return Err(budget());
                }
                if self.dispatch_ops.contains_key(&current) {
                    if group.is_some()
                        || coercion
                        || narrowing
                        || read
                        || typed
                        || self.fields.contains_key(&current)
                        || reports.consumers.contains_key(&current)
                        || point.kind != PointKind::Expr
                    {
                        return Err(invalid());
                    }
                    return Ok(Some(current));
                }
            }
            if usize::from(coercion)
                + usize::from(narrowing)
                + usize::from(read)
                + usize::from(typed)
                > 1
            {
                return Err(invalid());
            }
            if let Some(&(source_owner, _)) = reports.consumers.get(&current) {
                if group.is_some()
                    || coercion
                    || narrowing
                    || read
                    || typed
                    || source_owner != owner
                    || point.kind != PointKind::Expr
                {
                    return Err(invalid());
                }
                return Ok((!dispatch).then_some(current));
            }
            let block = point.block;
            let next = if let Some(group) = group {
                if coercion || narrowing || read || typed {
                    return Err(invalid());
                }
                self.qualified_group_input(current, owner, group, span)?
            } else if let Some(input) =
                self.forward_coercion_input(reports, current, owner, span)?
            {
                input
            } else if let Some(input) =
                self.unchanged_narrowing_input(reports, current, owner, span)?
            {
                input
            } else if let Some(input) =
                self.read_initializer_input(reports, current, owner, span)?
            {
                input
            } else if let Some(input) = self.read_receiver_input(reports, current, owner, span)? {
                input
            } else if let Some(input) =
                self.unchanged_ascription_input(reports, current, owner, span)?
            {
                input
            } else {
                return Ok(None);
            };
            if !self.flow.spend(
                self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + seen.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + 6,
            ) {
                return Err(budget());
            }
            if !block
                .and_then(|block| self.bodies.get(&block))
                .is_some_and(|body| body.owner == owner)
                || seen.contains(&current)
            {
                return Err(invalid());
            }
            if seen.len() >= limit {
                return Err(budget());
            }
            seen.insert(current);
            current = next;
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod validation;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod mixed;

#[cfg(test)]
mod forward;

#[cfg(test)]
mod narrowing;

#[cfg(test)]
mod narrow_limits;

#[cfg(test)]
mod read_limits;

#[cfg(test)]
mod dispatch;

#[cfg(test)]
mod ascriptions;

#[cfg(test)]
mod ascription_limits;
