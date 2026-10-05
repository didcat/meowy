use super::*;
use crate::check::dependencies::{
    edges::forward::{consumers::Slot, effects::Effect},
    emissions::Projection,
};
use std::collections::BTreeSet;

mod collection;
pub(in super::super) mod direct;
pub(in super::super) mod graph;
mod sources;

pub(crate) type Key = (hir::BlockId, usize, usize);
pub(crate) type Inputs = BTreeMap<Key, (usize, Input)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Input {
    pub(crate) candidate: Candidate,
    pub(crate) point: PointId,
    pub(crate) projection: Projection,
    pub(crate) source: Option<Slot>,
}

pub(super) struct Context<'a> {
    pub(super) reports: &'a Reports,
    pub(super) parts: usize,
    pub(super) blocks: BTreeSet<hir::BlockId>,
    pub(super) emissions: BTreeSet<PointId>,
    pub(super) sources: BTreeMap<PointId, hir::BlockId>,
}

impl<'a> Context<'a> {
    pub(super) fn new(reports: &'a Reports, parts: usize) -> Self {
        Self {
            reports,
            parts: parts.min(MAX_EDGES),
            blocks: BTreeSet::new(),
            emissions: BTreeSet::new(),
            sources: BTreeMap::new(),
        }
    }
}

impl Checker {
    pub(super) fn candidate_result(
        &mut self,
        ctx: &mut Context<'_>,
        id: hir::BlockId,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof candidate-input budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof candidate-input identity mismatch", span);
        if !self.flow.spend(
            ctx.reports.results.len().checked_ilog2().unwrap_or(0) as usize
                + ctx.blocks.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 3,
        ) {
            return Err(budget());
        }
        if ctx.blocks.contains(&id) {
            return Ok(());
        }
        let (owner, result) = ctx.reports.results.get(&id).ok_or_else(invalid)?;
        if ctx.parts == 0 {
            return Err(budget());
        }
        self.validate_result_report(ctx.reports, id, *owner, result, span)?;
        ctx.parts -= 1;
        ctx.blocks.insert(id);
        Ok(())
    }

    pub(super) fn candidate_input(
        &mut self,
        ctx: &mut Context<'_>,
        key: Key,
        span: Span,
    ) -> Result<(usize, Input)> {
        let budget = || Diagnostic::unsupported("proof candidate-input budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof candidate-input identity mismatch", span);
        let (block, slot, position) = key;
        self.candidate_result(ctx, block, span)?;
        let reports = ctx.reports;
        if !self.flow.spend(
            reports.results.len().checked_ilog2().unwrap_or(0) as usize
                + reports.effects.len().checked_ilog2().unwrap_or(0) as usize
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + ctx.emissions.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 9,
        ) {
            return Err(budget());
        }
        let (owner, result) = reports.results.get(&block).ok_or_else(invalid)?;
        let Some(Sources::Candidates(values)) =
            result.slots.as_ref().and_then(|slots| slots.get(slot))
        else {
            return Err(invalid());
        };
        let candidate = *values.get(position).ok_or_else(invalid)?;
        let Layout::Slots(layout) = &self.bodies.get(&block).ok_or_else(invalid)?.layout else {
            return Err(invalid());
        };
        let selected = layout.get(slot).ok_or_else(invalid)?;
        if selected.mutable || !matches!(selected.shape, Shape::Scalar(_)) {
            return Err(invalid());
        }
        let (source_owner, Effect::Emission(observed)) = reports
            .effects
            .get(&candidate.statement)
            .ok_or_else(invalid)?
        else {
            return Err(invalid());
        };
        if source_owner != owner {
            return Err(invalid());
        }
        if !ctx.emissions.contains(&candidate.statement) {
            if ctx.parts == 0 {
                return Err(budget());
            }
            self.validate_emission_report(reports, candidate.statement, *owner, observed, span)?;
            ctx.parts -= 1;
            ctx.emissions.insert(candidate.statement);
        }
        let target = observed.targets.get(candidate.target).ok_or_else(invalid)?;
        let Layout::Slots(layout) = &self.bodies[&block].layout else {
            return Err(invalid());
        };
        let selected = &layout[slot];
        if !self.flow.spend(
            selected.field.as_ref().map_or(0, String::len)
                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                + 8,
        ) {
            return Err(budget());
        }
        if target.id != candidate.emission
            || target.block != block
            || target.field != selected.field
            || observed.initialized.get(candidate.target) != Some(&true)
            || target.alias.is_some_and(|id| {
                self.proofs
                    .aliases
                    .get(&id)
                    .is_none_or(|alias| alias.mutable)
            })
        {
            return Err(invalid());
        }
        let mut input = Input {
            candidate,
            point: observed.input,
            projection: target.projection,
            source: None,
        };
        input.source = self.candidate_source_slot(ctx, *owner, &input, span)?;
        Ok((*owner, input))
    }
}

#[cfg(test)]
mod tests;
