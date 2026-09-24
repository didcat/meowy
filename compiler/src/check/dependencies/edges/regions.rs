use super::{Edge, PointKind, Port, Route};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir::PointId,
};

impl Checker {
    pub(crate) fn region_edges(&mut self, id: PointId, child: PointId, span: Span) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof region edge budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof region edge identity mismatch", span);
        if !self
            .flow
            .spend(self.region_edges.len().checked_ilog2().unwrap_or(0) as usize + 3)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if !matches!(
            point.kind,
            PointKind::Condition | PointKind::Then | PointKind::Else | PointKind::Stmt
        ) || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
        {
            return Err(invalid());
        }
        let source = self.points.get(child).ok_or_else(invalid)?;
        if source.parent != Some(id)
            || source.owner != point.owner
            || source.block != point.block
            || !source.complete
            || !matches!(
                source.kind,
                PointKind::Expr | PointKind::And | PointKind::Or | PointKind::Stmt
            )
            || (matches!(point.kind, PointKind::Condition | PointKind::Stmt)
                && source.kind == PointKind::Stmt)
        {
            return Err(invalid());
        }
        let edges = [
            Edge::new(Port::Entry(id), Port::Entry(child), Route::Next),
            Edge::new(Port::Normal(child), Port::Normal(id), Route::Next),
        ];
        if let Some(prior) = self.region_edges.get(&id) {
            return if *prior == edges {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if !self.edge_room(edges.len()) {
            return Err(budget());
        }
        self.region_edges.insert(id, edges);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
