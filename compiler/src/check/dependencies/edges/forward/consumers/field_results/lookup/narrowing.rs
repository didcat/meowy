use super::*;
use crate::check::dependencies::grouped::MAX_GROUPS;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) field: PointId,
    pub(crate) slot: Slot,
}

impl Checker {
    pub(in super::super::super::super) fn field_narrowing_source(
        &mut self,
        ctx: &mut Lookup<'_>,
        input: PointId,
        owner: usize,
        span: Span,
        limit: usize,
    ) -> Result<Option<Source>> {
        let budget = || Diagnostic::unsupported("proof field-narrowing budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof field-narrowing identity mismatch", span);
        if self.group_inputs.len() > MAX_GROUPS || !self.flow.spend(1) {
            return Err(budget());
        }
        let reports = ctx.reports;
        let mut seen = BTreeSet::new();
        let mut current = input;
        loop {
            let work = [
                self.fields.len(),
                self.narrowings.len(),
                self.group_inputs.len(),
                self.coercions.len(),
                self.local_reads.len(),
                self.typed_ops.len(),
                reports.consumers.len(),
                self.bodies.len(),
            ]
            .into_iter()
            .fold(10, |work, len| {
                work + len.checked_ilog2().unwrap_or(0) as usize
            });
            if !self
                .flow
                .spend(work + seen.len().checked_ilog2().unwrap_or(0) as usize * 2)
            {
                return Err(budget());
            }
            let point = self.points.get(current).ok_or_else(invalid)?;
            if !point.complete
                || point.owner != owner
                || point.span.start > point.span.end
                || !matches!(point.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                || !point
                    .block
                    .and_then(|block| self.bodies.get(&block))
                    .is_some_and(|body| body.owner == owner)
                || seen.contains(&current)
            {
                return Err(invalid());
            }
            let producers = [
                self.fields.contains_key(&current),
                self.narrowings.contains_key(&current),
                self.group_inputs.contains_key(&current),
                self.coercions.contains_key(&current),
                self.local_reads.contains_key(&current),
                self.typed_ops.contains_key(&current),
                reports.consumers.contains_key(&current),
            ];
            if producers.into_iter().filter(|present| *present).count() > 1 {
                return Err(invalid());
            }
            if let Some(slot) = self.field_result_source(ctx, current, owner, span)? {
                return Ok(Some(Source {
                    field: current,
                    slot,
                }));
            }
            let next = if let Some(&group) = self.group_inputs.get(&current) {
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
                self.unchanged_ascription_input(reports, current, owner, span)?
            {
                input
            } else {
                return Ok(None);
            };
            if seen.len() >= limit.min(MAX_GROUPS) {
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
mod limits;

#[cfg(test)]
mod groups;

#[cfg(test)]
mod group_faults;

#[cfg(test)]
mod group_limits;

#[cfg(test)]
mod forward;

#[cfg(test)]
mod forward_boundaries;

#[cfg(test)]
mod forward_limits;

#[cfg(test)]
mod ascriptions;
