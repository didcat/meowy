use super::{entries::Reports, *};
use crate::{
    check::dependencies::bodies::{Layout, completion::Shape},
    hir,
};

mod index;
pub(super) mod inputs;
mod slots;
mod validation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub(crate) emission: hir::EmitId,
    pub(crate) statement: PointId,
    pub(crate) target: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Sources {
    Unknown,
    Candidates(Vec<Candidate>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) consumer: Option<PointId>,
    pub(crate) slots: Option<Vec<Sources>>,
}

pub(crate) type Results = BTreeMap<hir::BlockId, (usize, Observed)>;

impl Checker {
    pub(super) fn result_sources(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<(Results, usize)> {
        self.result_sources_limited(reports, span, MAX_EDGES, reports.parts)
    }

    pub(self) fn result_sources_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
        mut parts: usize,
    ) -> Result<(Results, usize)> {
        let budget = || Diagnostic::unsupported("proof result-source budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof result-source identity mismatch", span);
        let limit = limit
            .checked_sub(reports.effects.len())
            .and_then(|room| room.checked_sub(reports.blocks.len()))
            .ok_or_else(budget)?;
        if !self.flow.spend(reports.blocks.len() + 1) {
            return Err(budget());
        }
        let index = self.result_source_index(reports, &mut parts, span)?;
        let mut results = Results::new();
        for (&id, (owner, observed)) in &reports.blocks {
            if !observed.result {
                continue;
            }
            if !self.validate_block_effect(reports, *owner, Port::BlockResult(id), span)? {
                return Err(invalid());
            }
            let body = &self.bodies[&id];
            if observed.parent != body.parent
                || observed.span != body.span
                || observed.completion != body.completion
            {
                return Err(invalid());
            }
            if results.len() >= limit
                || !self
                    .flow
                    .spend(results.len().checked_ilog2().unwrap_or(0) as usize + 3)
            {
                return Err(budget());
            }
            let (slots, left) =
                slots::collect(&body.layout, &index, id, &mut self.flow, parts, span)?;
            parts = left;
            results.insert(
                id,
                (
                    *owner,
                    Observed {
                        consumer: body.parent,
                        slots,
                    },
                ),
            );
        }
        Ok((results, parts))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
