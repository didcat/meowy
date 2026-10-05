use super::*;
use crate::check::dependencies::{
    edges::forward::consumers::{
        dispatch::Source as Dispatch,
        field_results::lookup::{Lookup, narrowing::Source},
        scalars::Source as Block,
    },
    grouped::MAX_GROUPS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Direct {
    pub(crate) point: PointId,
    pub(crate) source: Option<Source>,
    pub(crate) block: Option<Block>,
    pub(crate) dispatch: Option<Dispatch>,
}

pub(crate) type Directs = BTreeMap<Key, (usize, Direct)>;

mod qualified;

impl Checker {
    pub(in super::super::super) fn direct_sources(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<(Directs, usize)> {
        self.direct_sources_limited(reports, span, MAX_EDGES, reports.parts)
    }

    pub(super) fn direct_sources_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
        parts: usize,
    ) -> Result<(Directs, usize)> {
        let budget = || Diagnostic::unsupported("proof direct-source budget exhausted", span);
        let limit = limit
            .min(MAX_EDGES)
            .checked_sub(reports.effects.len())
            .and_then(|room| room.checked_sub(reports.blocks.len()))
            .and_then(|room| room.checked_sub(reports.results.len()))
            .and_then(|room| room.checked_sub(reports.consumers.len()))
            .and_then(|room| room.checked_sub(reports.eligible.len()))
            .and_then(|room| room.checked_sub(reports.initializers.len()))
            .and_then(|room| room.checked_sub(reports.slot_uses.len()))
            .and_then(|room| room.checked_sub(reports.candidate_inputs.len()))
            .and_then(|room| room.checked_sub(reports.field_results.len()))
            .ok_or_else(budget)?;
        let (graph, parts) = self.candidate_graph_limited(reports, span, parts)?;
        if !self.flow.spend(graph.inputs.len() + 1) {
            return Err(budget());
        }
        let mut ctx = Lookup::new(reports, parts);
        let mut sources = Directs::new();
        for (&key, &(owner, input)) in graph.inputs {
            if input.projection != Projection::Value {
                continue;
            }
            if sources.len() >= limit
                || !self
                    .flow
                    .spend(sources.len().checked_ilog2().unwrap_or(0) as usize * 2 + 2)
            {
                return Err(budget());
            }
            let source = self.direct_source(&mut ctx, input.point, owner, span)?;
            sources.insert(key, (owner, source));
        }
        Ok((sources, ctx.parts))
    }

    pub(super) fn direct_source(
        &mut self,
        ctx: &mut Lookup<'_>,
        point: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Direct> {
        let source = self.field_narrowing_source(ctx, point, owner, span, MAX_GROUPS)?;
        let block = if source.is_none() {
            self.scalar_block_source(ctx.reports, point, owner, span)?
        } else {
            None
        };
        let dispatch = if source.is_none() && block.is_none() {
            self.scalar_dispatch_source(ctx.reports, point, owner, span)?
        } else {
            None
        };
        Ok(Direct {
            point,
            source,
            block,
            dispatch,
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod blocks;

#[cfg(test)]
mod block_limits;
