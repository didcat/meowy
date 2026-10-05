use super::super::graph::{Graph, Walk};
use super::*;

pub(crate) struct Expanded<'a> {
    pub(super) graph: Graph<'a>,
    pub(super) sources: &'a Directs,
}

impl Checker {
    pub(in super::super::super::super) fn direct_walk_report(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<(Walk, usize)> {
        self.direct_walk_limited(reports, span, reports.parts)
    }

    pub(super) fn direct_walk_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Walk, usize)> {
        let (view, parts) = self.direct_graph(reports, span, parts)?;
        view.graph
            .forest_sources(Some(view.sources), &mut self.flow, span, parts)
    }

    pub(in super::super::super::super) fn direct_graph<'a>(
        &mut self,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Expanded<'a>, usize)> {
        let budget = || Diagnostic::unsupported("proof direct-graph budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof direct-graph identity mismatch", span);
        if reports.direct_sources.len() > MAX_EDGES {
            return Err(budget());
        }
        let (graph, parts) = self.candidate_graph_limited(reports, span, parts)?;
        if !self.flow.spend(graph.inputs.len() + 1) {
            return Err(budget());
        }
        let mut ctx = Lookup::new(reports, parts);
        let mut count = 0usize;
        for (key, &(owner, input)) in graph.inputs {
            if input.projection != Projection::Value {
                continue;
            }
            if !self
                .flow
                .spend(reports.direct_sources.len().checked_ilog2().unwrap_or(0) as usize + 3)
            {
                return Err(budget());
            }
            let &(stored_owner, direct) = reports.direct_sources.get(key).ok_or_else(invalid)?;
            if stored_owner != owner
                || direct.point != input.point
                || direct.source
                    != self.field_narrowing_source(
                        &mut ctx,
                        input.point,
                        owner,
                        span,
                        MAX_GROUPS,
                    )?
            {
                return Err(invalid());
            }
            count += 1;
        }
        if count != reports.direct_sources.len() {
            return Err(invalid());
        }
        Ok((
            Expanded {
                graph,
                sources: &reports.direct_sources,
            },
            ctx.parts,
        ))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod forest;
