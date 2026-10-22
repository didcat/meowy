use super::*;
use crate::check::dependencies::bodies::{Fact, MAX_BODY_FACTS};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Leave {
    pub(super) owner: usize,
    pub(super) block: hir::BlockId,
    pub(super) target: hir::BlockId,
    pub(super) control: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Sources {
    pub(super) leaves: BTreeMap<PointId, Leave>,
    pub(super) targets: HashSet<hir::BlockId>,
    pub(super) unknown: HashSet<hir::BlockId>,
    pub(super) incomplete: HashSet<usize>,
}

impl Checker {
    pub(super) fn scalar_exit_sources(
        &mut self,
        span: Span,
        parts: usize,
    ) -> Result<(Sources, usize)> {
        if self.bodies.len() > MAX_EDGES || !self.flow.spend(self.bodies.len() + 1) {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        let mut sources = Sources::default();
        let mut facts = 0usize;
        for (&block, body) in &self.bodies {
            facts = facts
                .checked_add(body.facts.len())
                .ok_or_else(|| budget(span))?;
            if facts > MAX_BODY_FACTS || !self.flow.spend(body.facts.len() + 2) {
                return Err(budget(span));
            }
            if body.facts.len() != body.sources.len() {
                return Err(invalid(span));
            }
            for ((fact, at), point) in body.facts.iter().zip(&body.sources) {
                if let Fact::Block(child) = fact {
                    if !self
                        .flow
                        .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 2)
                    {
                        return Err(budget(span));
                    }
                    match self.bodies.get(child) {
                        Some(child) if child.owner != body.owner => return Err(invalid(span)),
                        None if sources.incomplete.insert(body.owner) => {
                            parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                        }
                        _ => (),
                    }
                }
                let Fact::Leave(target) = fact else { continue };
                if !self.flow.spend(
                    self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                        + self.scope_exits.len().checked_ilog2().unwrap_or(0) as usize
                        + sources.leaves.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + 8,
                ) {
                    return Err(budget(span));
                }
                if self
                    .bodies
                    .get(target)
                    .is_none_or(|target| target.owner != body.owner)
                {
                    return Err(invalid(span));
                }
                if sources.targets.insert(*target) {
                    parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                }
                let Some(point) = *point else {
                    if sources.unknown.insert(*target) {
                        parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                    }
                    continue;
                };
                let source = self.points.get(point).ok_or_else(|| invalid(span))?;
                let exit = self.scope_exits.get(&point).ok_or_else(|| invalid(span))?;
                if !source.complete
                    || source.kind != PointKind::Stmt
                    || source.owner != body.owner
                    || source.block != Some(block)
                    || exit.owner != body.owner
                    || exit.span != *at
                    || exit.edge != Edge::new(Port::Entry(point), Port::Leave(*target), Route::Exit)
                {
                    return Err(invalid(span));
                }
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                if sources
                    .leaves
                    .insert(
                        point,
                        Leave {
                            owner: body.owner,
                            block,
                            target: *target,
                            control: exit.control,
                        },
                    )
                    .is_some()
                {
                    return Err(invalid(span));
                }
            }
        }
        if self.scope_exits.len() > MAX_EDGES || !self.flow.spend(self.scope_exits.len() + 1) {
            return Err(budget(span));
        }
        for (&point, exit) in &self.scope_exits {
            let Port::Leave(target) = exit.edge.to else {
                continue;
            };
            if !self
                .flow
                .spend(sources.leaves.len().checked_ilog2().unwrap_or(0) as usize + 3)
            {
                return Err(budget(span));
            }
            if !sources.leaves.contains_key(&point)
                && !sources.unknown.contains(&target)
                && !sources.incomplete.contains(&exit.owner)
            {
                return Err(invalid(span));
            }
        }
        Ok((sources, parts))
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-exit budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-exit identity mismatch", span)
}

#[cfg(test)]
mod tests;
