use super::*;
use crate::check::dependencies::edges::forward::results::inputs::graph::{Visit, Walk};

#[derive(Debug)]
pub(crate) struct Forest<'a> {
    pub(super) reports: &'a Reports,
    pub(crate) walk: &'a Walk,
    pub(super) index: BTreeMap<PointId, usize>,
}

impl Checker {
    pub(in super::super) fn argument_forest_view<'a>(
        &mut self,
        reports: &'a Reports,
        span: Span,
        limit: usize,
        parts: usize,
    ) -> Result<(Forest<'a>, usize)> {
        let budget =
            || Diagnostic::unsupported("proof argument-forest view budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof argument-forest view identity mismatch", span);
        let visits = &reports.argument_walk.visits;
        if visits.len() > MAX_EDGES || !self.flow.spend(visits.len() * 2 + 1) {
            return Err(budget());
        }
        let (expected, mut parts) = self.argument_walk_report(reports, span, parts)?;
        if expected != reports.argument_walk {
            return Err(invalid());
        }
        let mut index = BTreeMap::new();
        for (position, visit) in visits.iter().enumerate() {
            let Visit::Argument(_, direct) = visit else {
                continue;
            };
            if index.len() >= limit.min(MAX_EDGES)
                || !self
                    .flow
                    .spend(index.len().checked_ilog2().unwrap_or(0) as usize * 2 + 2)
            {
                return Err(budget());
            }
            parts = parts.checked_sub(1).ok_or_else(budget)?;
            if index.insert(direct.point, position).is_some() {
                return Err(invalid());
            }
        }
        Ok((
            Forest {
                reports,
                walk: &reports.argument_walk,
                index,
            },
            parts,
        ))
    }
}

#[cfg(test)]
mod tests;
