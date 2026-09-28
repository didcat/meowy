use super::*;
use crate::check::queries::Prepared;

impl Checker {
    pub(crate) fn pending_declaration_endpoint(
        &mut self,
        id: PointId,
        prepared: Prepared,
        span: Span,
    ) -> Result<()> {
        let invalid = || {
            Diagnostic::unsupported("proof pending declaration endpoint identity mismatch", span)
        };
        if !self.flow.spend(
            self.endpoints.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.sites.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 16,
        ) {
            return Err(Diagnostic::unsupported(
                "proof pending declaration endpoint budget exhausted",
                span,
            ));
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let query = self.queries.get(prepared.id).ok_or_else(invalid)?;
        let source = self.points.get(query.point).ok_or_else(invalid)?;
        if point.kind != PointKind::Stmt
            || point.owner != self.owner
            || (!point.complete && self.point != Some(id))
            || query.owner != self.owner
            || source.owner != query.owner
            || source.kind != PointKind::Query
            || !source.complete
            || source.site != query.site
            || source.span != query.span
            || (source.parent == Some(id)) != prepared.created
            || prepared.created && (source.block != point.block || source.site != point.site)
        {
            return Err(invalid());
        }
        for (at, current) in [(point, true), (source, prepared.created)] {
            if let Some(site) = at.site {
                let item = self.sites.get(&site).ok_or_else(invalid)?;
                let checked = item.complete
                    && item.point.is_some_and(|id| {
                        self.points.get(id).is_some_and(|point| {
                            point.kind == PointKind::Stmt
                                && point.complete
                                && point.owner == item.owner
                                && point.block == item.block
                                && point.site == Some(site)
                        })
                    });
                let active = current && self.point == Some(id) && self.site == Some(site);
                if item.owner != at.owner || item.block != at.block || !(checked || active) {
                    return Err(invalid());
                }
            }
        }
        let root = self.query_budgets.get(query.root).ok_or_else(invalid)?;
        let budget = match root {
            Some(budget) => budget,
            None => {
                &self
                    .type_work
                    .as_ref()
                    .filter(|work| work.query_root == Some(query.root))
                    .ok_or_else(invalid)?
                    .logical
            }
        };
        if budget.failure.is_some() {
            return Err(invalid());
        }
        self.publish_declaration_endpoint(id, span, "pending declaration")
    }
}

#[cfg(test)]
mod tests;
