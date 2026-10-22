use super::*;

impl Checker {
    pub(super) fn scalar_stopped_region(
        &mut self,
        reports: &Reports,
        graph: &graph::Graph<'_>,
        point: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<PointId>> {
        if !self
            .flow
            .spend(self.region_edges.len().checked_ilog2().unwrap_or(0) as usize + 5)
        {
            return Err(budget(span));
        }
        let source = self.points.get(point).ok_or_else(|| invalid(span))?;
        let Some(parent) = source.parent else {
            return Ok(None);
        };
        let region = self.points.get(parent).ok_or_else(|| invalid(span))?;
        if !matches!(region.kind, PointKind::Then | PointKind::Else) {
            return Ok(None);
        }
        if parent >= point
            || !region.complete
            || region.owner != owner
            || region.block != source.block
        {
            return Err(invalid(span));
        }
        let Some(edges) = self.region_edges.get(&parent).copied() else {
            return Ok(None);
        };
        for edge in edges {
            self.scalar_completion_edge(reports, Family::Region, edge, span)?;
        }
        if edges
            != [
                Edge::new(Port::Entry(parent), Port::Entry(point), Route::Next),
                Edge::new(Port::Normal(point), Port::Normal(parent), Route::Next),
            ]
        {
            return Ok(None);
        }
        if graph.nodes.contains_key(&Port::Normal(parent)) {
            return Err(invalid(span));
        }
        Ok(Some(parent))
    }
}

#[cfg(test)]
mod tests;
