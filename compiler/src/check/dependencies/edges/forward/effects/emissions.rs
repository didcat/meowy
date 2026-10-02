use super::*;
use crate::check::dependencies::emissions::{Composition, MAX_TARGETS, Projection, Target};
use std::collections::BTreeSet;

mod report;
pub(crate) use report::Observed;

pub(in super::super) type Stage = (PointId, Option<usize>);

pub(super) fn name_bytes(
    targets: &[Target],
    flow: &mut crate::flow::Flow,
    span: Span,
) -> Result<usize> {
    let budget = || Diagnostic::unsupported("proof emission-effect budget exhausted", span);
    if targets.len() > MAX_TARGETS || !flow.spend(targets.len() + 1) {
        return Err(budget());
    }
    let bytes = targets.iter().try_fold(0usize, |total, target| {
        total
            .checked_add(target.field.as_ref().map_or(0, String::len))
            .ok_or_else(budget)
    })?;
    if bytes > MAX_EDGES {
        return Err(budget());
    }
    Ok(bytes)
}

impl Checker {
    pub(in super::super) fn emission_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let budget = || Diagnostic::unsupported("proof emission-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof emission-effect identity mismatch", span);
        if !matches!(port, Port::Emission(_) | Port::Normal(_)) {
            return Ok(None);
        }
        if !self.flow.spend(
            self.emission_sources.len().checked_ilog2().unwrap_or(0) as usize
                + self.emissions.len().checked_ilog2().unwrap_or(0) as usize
                + 2,
        ) {
            return Err(budget());
        }
        let (id, part) = match port {
            Port::Emission(target) => {
                let &(id, part) = self.emission_sources.get(&target).ok_or_else(invalid)?;
                (id, Some(part))
            }
            Port::Normal(id) => (id, None),
            _ => unreachable!(),
        };
        let Some(op) = self.emissions.get(&id) else {
            return if part.is_none() {
                Ok(None)
            } else {
                Err(invalid())
            };
        };
        let bytes = name_bytes(&op.targets, &mut self.flow, span)?;
        let work = [
            self.emission_sources.len(),
            self.proofs.emissions.len(),
            self.proofs.aliases.len(),
        ]
        .into_iter()
        .fold(14, |cost, len| {
            cost + len.checked_ilog2().unwrap_or(0) as usize
        });
        if !self.flow.spend(
            op.targets.len() * work
                + bytes * (op.targets.len().checked_ilog2().unwrap_or(0) as usize + 3)
                + self.sites.len().checked_ilog2().unwrap_or(0) as usize
                + 16,
        ) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let site = point.site.ok_or_else(invalid)?;
        let block = point.block.ok_or_else(invalid)?;
        let first = op.targets.first().ok_or_else(invalid)?;
        if op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Stmt
            || point.span != op.span
            || site >= self.statements
            || op.input == id
            || !self.points.get(op.input).is_some_and(|input| {
                input.complete
                    && input.owner == owner
                    && input.parent == Some(id)
                    && input.block == point.block
                    && input.site == point.site
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
            || match op.composed {
                Some(Composition { local, count }) => {
                    local >= reports.locals
                        || count >= MAX_TARGETS
                        || op.targets.len() != count + 1
                        || self.proofs.aliases.contains_key(&local)
                }
                None => op.targets.len() != 1,
            }
        {
            return Err(invalid());
        }
        let mut names = BTreeSet::new();
        let mut edges = op.edges.iter();
        if edges.next()
            != Some(&Edge::new(
                Port::Entry(id),
                Port::Entry(op.input),
                Route::Next,
            ))
        {
            return Err(invalid());
        }
        let mut from = Port::Normal(op.input);
        for (index, target) in op.targets.iter().enumerate() {
            if target.block != first.block
                || !self.proofs.emissions.contains_key(&target.id)
                || self.emission_sources.get(&target.id) != Some(&(id, index))
                || target
                    .field
                    .as_ref()
                    .is_some_and(|name| name.is_empty() || !names.insert(name.as_str()))
                || match op.composed {
                    Some(_) if index == 0 => {
                        target.projection != Projection::Primary || target.field.is_some()
                    }
                    Some(_) => {
                        target.projection != Projection::Field(index - 1) || target.field.is_none()
                    }
                    None => target.projection != Projection::Value,
                }
            {
                return Err(invalid());
            }
            match (target.alias, target.storage) {
                (Some(alias), Some(storage)) if op.composed.is_none() => {
                    if alias >= reports.locals
                        || storage >= reports.locals
                        || !self.proofs.aliases.get(&alias).is_some_and(|alias| {
                            alias.emission == target.id
                                && alias.target == target.block
                                && alias.root == storage
                                && target.field.as_ref() == Some(&alias.field)
                        })
                    {
                        return Err(invalid());
                    }
                }
                (None, None) => (),
                _ => return Err(invalid()),
            }
            let to = Port::Emission(target.id);
            if edges.next() != Some(&Edge::new(from, to, Route::Next)) {
                return Err(invalid());
            }
            from = to;
        }
        if edges.next() != Some(&Edge::new(from, Port::Normal(id), Route::Next))
            || edges.next().is_some()
            || part.is_some_and(|part| {
                op.targets
                    .get(part)
                    .is_none_or(|target| port != Port::Emission(target.id))
            })
        {
            return Err(invalid());
        }
        let target = first.block;
        self.emission_statement_site(id, site, block, owner, span)?;
        self.emission_target_scope(block, target, owner, span)?;
        Ok(Some((id, part)))
    }

    pub(self) fn emission_statement_site(
        &mut self,
        mut id: PointId,
        site: crate::hir::StatementId,
        block: crate::hir::BlockId,
        owner: usize,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof emission-effect identity mismatch", span);
        let site_id = site;
        let site = self.sites.get(&site).ok_or_else(invalid)?;
        let root = site.point.ok_or_else(invalid)?;
        if !site.complete || site.owner != owner || site.block != Some(block) {
            return Err(invalid());
        }
        for _ in 0..=self.points.len() {
            if !self.flow.spend(4) {
                return Err(Diagnostic::unsupported(
                    "proof emission-effect budget exhausted",
                    span,
                ));
            }
            let point = self.points.get(id).ok_or_else(invalid)?;
            if !point.complete
                || point.owner != owner
                || point.block != Some(block)
                || point.site != Some(site_id)
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

    pub(self) fn emission_target_scope(
        &mut self,
        mut block: crate::hir::BlockId,
        target: crate::hir::BlockId,
        owner: usize,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof emission-effect identity mismatch", span);
        for _ in 0..=self.bodies.len() {
            if !self
                .flow
                .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 4)
            {
                return Err(Diagnostic::unsupported(
                    "proof emission-effect budget exhausted",
                    span,
                ));
            }
            let body = self.bodies.get(&block).ok_or_else(invalid)?;
            if body.owner != owner {
                return Err(invalid());
            }
            if block == target {
                return Ok(());
            }
            let parent = body
                .parent
                .and_then(|id| self.points.get(id))
                .ok_or_else(invalid)?;
            if !parent.complete || parent.owner != owner {
                return Err(invalid());
            }
            block = parent.block.ok_or_else(invalid)?;
        }
        Err(invalid())
    }
}

#[cfg(test)]
mod validation;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod limits;
