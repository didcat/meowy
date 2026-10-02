use super::{entries::Reports, *};
use crate::{
    check::dependencies::bodies::{Layout, completion::Shape},
    hir,
};

mod index;

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
            let slots = match &body.layout {
                Layout::Stopped => return Err(invalid()),
                Layout::Unknown => None,
                Layout::Slots(slots) => {
                    parts = parts.checked_sub(slots.len()).ok_or_else(budget)?;
                    let mut sources = Vec::with_capacity(slots.len());
                    for slot in slots {
                        if !self.flow.spend(
                            (slot.field.as_ref().map_or(0, String::len) + 1)
                                * (index.len().checked_ilog2().unwrap_or(0) as usize + 3),
                        ) {
                            return Err(budget());
                        }
                        let row = index.get(&(id, slot.field.as_deref()));
                        let source = if slot.mutable
                            || !matches!(slot.shape, Shape::Scalar(_))
                            || row.is_some_and(|row| row.mutable)
                        {
                            Sources::Unknown
                        } else {
                            let values = row.map_or(&[][..], |row| row.values.as_slice());
                            parts = parts.checked_sub(values.len()).ok_or_else(budget)?;
                            if !self.flow.spend(values.len() + 1) {
                                return Err(budget());
                            }
                            Sources::Candidates(values.to_vec())
                        };
                        sources.push(source);
                    }
                    Some(sources)
                }
            };
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
