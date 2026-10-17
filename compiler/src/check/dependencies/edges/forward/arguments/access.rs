use super::{forests::Forest, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;
use crate::check::dependencies::{Invocation, sequences::MAX_ITEMS};

pub(crate) type Input<'a> = (hir::LocalId, PointId, Option<&'a Argument>, Option<usize>);

impl Checker {
    pub(in super::super) fn call_argument_forest<'a>(
        &'a mut self,
        forest: &'a Forest<'a>,
        site: hir::CallId,
        span: Span,
    ) -> Result<(
        &'a Invocation,
        impl ExactSizeIterator<Item = Input<'a>> + 'a,
    )> {
        let budget = || Diagnostic::unsupported("proof call-forest access budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof call-forest access identity mismatch", span);
        if !self
            .flow
            .spend(self.invocations.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let count = self.invocations.get(&site).ok_or_else(invalid)?.args.len();
        let rows = &forest.reports.arguments;
        let index = &forest.index;
        if count > MAX_ITEMS
            || !self.flow.spend(
                count
                    * ((rows.len().checked_ilog2().unwrap_or(0) as usize
                        + index.len().checked_ilog2().unwrap_or(0) as usize)
                        * 2
                        + 12)
                    + 1,
            )
        {
            return Err(budget());
        }
        let inputs = self.checked_call_inputs(forest.reports, site, span)?;
        for (position, (param, point)) in inputs.pairs.clone().enumerate() {
            let row = rows.get(&point);
            let marker = index.get(&point);
            if row.is_some_and(|row| {
                row.site != site
                    || row.position != position
                    || row.param != param
                    || row.owner != inputs.call.owner
            }) || row.is_some_and(|row| row.normal) != marker.is_some()
            {
                return Err(invalid());
            }
            if marker.is_some_and(|&marker| {
                !matches!(forest.walk.visits.get(marker), Some(Visit::Argument(owner, direct))
                    if *owner == inputs.call.owner && direct.point == point)
            }) {
                return Err(invalid());
            }
        }
        Ok((
            inputs.call,
            inputs.pairs.map(move |(param, point)| {
                (param, point, rows.get(&point), index.get(&point).copied())
            }),
        ))
    }

    pub(in super::super) fn qualify_call_forests(
        &mut self,
        reports: &Reports,
        span: Span,
        parts: usize,
    ) -> Result<usize> {
        let (forest, parts) = self.argument_forest_view(reports, span, MAX_EDGES, parts)?;
        let calls = self.call_effect_index(span)?;
        for site in calls.into_values() {
            let _ = self.call_argument_forest(&forest, site, span)?;
        }
        Ok(parts)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod shapes;
