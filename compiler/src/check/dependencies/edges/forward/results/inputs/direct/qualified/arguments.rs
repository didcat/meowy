use super::*;
use crate::check::dependencies::edges::forward::arguments::sources::Sources as Arguments;

impl Checker {
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
