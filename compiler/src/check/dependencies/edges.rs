use super::PointKind;
use crate::{ast::Span, check::Checker, check::Result, diagnostic::Diagnostic, hir::PointId};

pub(crate) const MAX_EDGES: usize = 262_144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Port {
    Entry(PointId),
    Normal(PointId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    Next,
    True,
    False,
    Join,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Edge {
    pub(crate) from: Port,
    pub(crate) to: Port,
    pub(crate) route: Route,
}

impl Edge {
    pub(crate) const fn new(from: Port, to: Port, route: Route) -> Self {
        Self { from, to, route }
    }
}

impl Checker {
    pub(crate) fn branch_edges(
        &mut self,
        id: PointId,
        condition: PointId,
        then: PointId,
        otherwise: PointId,
        span: Span,
    ) -> Result<()> {
        use Port::{Entry, Normal};
        use Route::{False, Join, Next, True};
        let budget = || Diagnostic::unsupported("proof branch edge budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof branch edge identity mismatch", span);
        if !self
            .flow
            .spend(self.branch_edges.len().checked_ilog2().unwrap_or(0) as usize + 5)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if !matches!(
            point.kind,
            PointKind::Match | PointKind::And | PointKind::Or
        ) || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
        {
            return Err(invalid());
        }
        for (child, kind) in [
            (condition, PointKind::Condition),
            (then, PointKind::Then),
            (otherwise, PointKind::Else),
        ] {
            if !self.points.get(child).is_some_and(|child| {
                child.kind == kind
                    && child.parent == Some(id)
                    && child.owner == point.owner
                    && child.block == point.block
                    && child.complete
            }) {
                return Err(invalid());
            }
        }
        let skipped = if point.kind == PointKind::Or {
            then
        } else {
            otherwise
        };
        let edges = [
            Edge::new(Entry(id), Entry(condition), Next),
            Edge::new(Normal(condition), Entry(then), True),
            Edge::new(Normal(condition), Entry(otherwise), False),
            Edge::new(Normal(then), Normal(id), Join),
            Edge::new(Normal(otherwise), Normal(id), Join),
            Edge::new(Entry(skipped), Normal(skipped), Next),
        ];
        if let Some(prior) = self.branch_edges.get(&id) {
            return if *prior == edges {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        if self.branch_edges.len() >= MAX_EDGES / edges.len() {
            return Err(budget());
        }
        self.branch_edges.insert(id, edges);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod logic;
