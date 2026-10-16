use super::*;
use crate::check::dependencies::edges::forward::arguments::sources::Sources as Arguments;

impl Expanded<'_> {
    pub(in super::super::super::super::super) fn argument_forest(
        &self,
        args: &Arguments,
        flow: &mut crate::flow::Flow,
        span: Span,
        parts: usize,
    ) -> Result<(Walk, usize)> {
        self.graph
            .argument_forest(args, self.sources, flow, span, parts)
    }
}

impl Checker {
    pub(in super::super::super::super::super) fn argument_walk_report(
        &mut self,
        reports: &Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Walk, usize)> {
        if reports.argument_sources.is_empty() {
            let (_, parts) = self.argument_source_view(reports, span, parts)?;
            return Ok((Walk::default(), parts));
        }
        let (graph, sources, parts) = self.argument_graph(reports, span, parts)?;
        graph.argument_forest(sources, &mut self.flow, span, parts)
    }

    pub(in super::super::super::super::super) fn argument_graph<'a>(
        &mut self,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Expanded<'a>, &'a Arguments, usize)> {
        let (sources, parts) = self.argument_source_view(reports, span, parts)?;
        let (graph, parts) = self.direct_graph(reports, span, parts)?;
        Ok((graph, sources, parts))
    }
}

#[cfg(test)]
mod tests;
