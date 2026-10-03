use super::{effects::Effect, entries::Reports, *};
use crate::{check::dependencies::OperationKind, hir};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Binding {
    pub(super) local: hir::LocalId,
    pub(super) storage: hir::LocalId,
    pub(super) input: Option<PointId>,
}

impl Checker {
    pub(super) fn validate_bindings(&mut self, reports: &Reports, span: Span) -> Result<()> {
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof binding budget exhausted",
                span,
            ));
        }
        for (&id, (owner, effect)) in &reports.effects {
            if matches!(effect, Effect::Storage { .. }) {
                self.binding_effect(reports, id, *owner, span)?;
            }
        }
        Ok(())
    }

    pub(super) fn binding_effect(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<Binding>> {
        let budget = || Diagnostic::unsupported("proof binding budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof binding identity mismatch", span);
        if !self
            .flow
            .spend(reports.effects.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some((
            reported,
            Effect::Storage {
                kind,
                local,
                storage,
                input,
                control,
            },
        )) = reports.effects.get(&id)
        else {
            return Ok(None);
        };
        if !self
            .flow
            .spend(self.operations.len().checked_ilog2().unwrap_or(0) as usize + 7)
        {
            return Err(budget());
        }
        let op = self.operations.get(&id).ok_or_else(invalid)?;
        if *reported != owner
            || op.owner != owner
            || *kind != op.kind
            || *local != op.local
            || *storage != op.storage
            || *input != op.input
            || *control != op.control
        {
            return Err(invalid());
        }
        if *kind != OperationKind::Bind {
            return Ok(None);
        }
        if !self.flow.spend(
            reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.bindings.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + self.sites.len().checked_ilog2().unwrap_or(0) as usize
                + 22,
        ) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let block = point.block.ok_or_else(invalid)?;
        let site = point.site.ok_or_else(invalid)?;
        let root = self
            .proofs
            .aliases
            .get(local)
            .map_or(*local, |alias| alias.root);
        if !point.complete
            || point.kind != PointKind::Stmt
            || point.owner != owner
            || point.span != op.span
            || point.span.start > point.span.end
            || *local >= reports.locals
            || *storage >= reports.locals
            || *storage != root
            || !self.proofs.bindings.contains_key(local)
            || !self.proofs.bindings.contains_key(storage)
            || reports.index.operations.get(&id) != Some(&owner)
            || !self
                .bodies
                .get(&block)
                .is_some_and(|body| body.owner == owner)
        {
            return Err(invalid());
        }
        let end = Edge::new(Port::Operation(id), Port::Normal(id), Route::Next);
        let mut edges = [end; 3];
        let count = if let Some(input) = *input {
            let child = self.points.get(input).ok_or_else(invalid)?;
            if input == id
                || !child.complete
                || child.owner != owner
                || child.parent != Some(id)
                || child.block != point.block
                || child.site != point.site
                || !matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                || child.span.start > child.span.end
                || child.span.start < point.span.start
                || child.span.end > point.span.end
            {
                return Err(invalid());
            }
            edges[0] = Edge::new(Port::Entry(id), Port::Entry(input), Route::Next);
            edges[1] = Edge::new(Port::Normal(input), Port::Operation(id), Route::Next);
            3
        } else {
            1
        };
        if op.edges.as_slice() != &edges[..count] {
            return Err(invalid());
        }
        let binding = Binding {
            local: *local,
            storage: *storage,
            input: *input,
        };
        self.binding_site(id, site, block, owner, span)?;
        Ok(Some(binding))
    }

    pub(self) fn binding_site(
        &mut self,
        mut id: PointId,
        site: hir::StatementId,
        block: hir::BlockId,
        owner: usize,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof binding identity mismatch", span);
        let site_id = site;
        let site = self.sites.get(&site).ok_or_else(invalid)?;
        let root = site.point.ok_or_else(invalid)?;
        if site_id >= self.statements
            || !site.complete
            || site.owner != owner
            || site.block != Some(block)
            || site.span.start > site.span.end
        {
            return Err(invalid());
        }
        for _ in 0..=self.points.len() {
            if !self.flow.spend(8) {
                return Err(Diagnostic::unsupported(
                    "proof binding budget exhausted",
                    span,
                ));
            }
            let point = self.points.get(id).ok_or_else(invalid)?;
            if !point.complete
                || point.owner != owner
                || point.block != Some(block)
                || point.site != Some(site_id)
                || point.span.start > point.span.end
                || point.span.start < site.span.start
                || point.span.end > site.span.end
            {
                return Err(invalid());
            }
            if id == root {
                return if point.kind == PointKind::Stmt && point.span == site.span {
                    Ok(())
                } else {
                    Err(invalid())
                };
            }
            id = point.parent.ok_or_else(invalid)?;
        }
        Err(invalid())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
