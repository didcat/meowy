use super::*;
use crate::check::dependencies::grouped::Group;

impl Checker {
    pub(in super::super::super) fn qualified_group_input(
        &mut self,
        current: PointId,
        owner: usize,
        group: Group,
        span: Span,
    ) -> Result<PointId> {
        let budget = || Diagnostic::unsupported("proof group-consumer budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof group-consumer identity mismatch", span);
        if !self
            .flow
            .spend(self.region_edges.len().checked_ilog2().unwrap_or(0) as usize + 10)
        {
            return Err(budget());
        }
        let point = self.points.get(current).ok_or_else(invalid)?;
        let child = self.points.get(group.input).ok_or_else(invalid)?;
        let edges = [
            Edge::new(Port::Entry(current), Port::Entry(group.input), Route::Next),
            Edge::new(
                Port::Normal(group.input),
                Port::Normal(current),
                Route::Next,
            ),
        ];
        if point.kind != PointKind::Expr
            || group.owner != owner
            || group.span != point.span
            || group.block != point.block
            || group.input == current
            || !child.complete
            || child.owner != owner
            || child.parent != Some(current)
            || child.block != point.block
            || child.span.start > child.span.end
            || child.span.start < point.span.start
            || child.span.end > point.span.end
            || !matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            || self.region_edges.get(&current) != Some(&edges)
        {
            return Err(invalid());
        }
        Ok(group.input)
    }
}
