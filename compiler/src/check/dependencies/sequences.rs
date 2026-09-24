use super::{
    PointKind,
    edges::{Edge, Port, Route},
};
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};
use std::collections::BTreeSet;

pub(crate) const MAX_ITEMS: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Source {
    Block(hir::BlockId),
    Expr(hir::PointId),
    Stmt(hir::PointId),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Sequence {
    pub(crate) owner: usize,
    pub(crate) items: Vec<Option<hir::PointId>>,
    pub(crate) edges: Vec<Edge>,
}

impl Checker {
    pub(crate) fn sequence(
        &mut self,
        source: Source,
        items: Vec<Option<hir::PointId>>,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof sequence budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof sequence identity mismatch", span);
        if items.len() > MAX_ITEMS
            || !self.flow.spend(
                items.len()
                    * (self.sites.len().checked_ilog2().unwrap_or(0) as usize
                        + items.len().checked_ilog2().unwrap_or(0) as usize
                        + 3)
                    + self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + 2,
            )
        {
            return Err(budget());
        }
        let (owner, block) = match source {
            Source::Block(block) => (
                self.bodies.get(&block).ok_or_else(invalid)?.owner,
                Some(block),
            ),
            Source::Expr(id) | Source::Stmt(id) => {
                let point = self.points.get(id).ok_or_else(invalid)?;
                let kind = if matches!(source, Source::Stmt(_)) {
                    PointKind::Stmt
                } else {
                    PointKind::Expr
                };
                if point.kind != kind || (!point.complete && self.point != Some(id)) {
                    return Err(invalid());
                }
                (point.owner, point.block)
            }
        };
        if owner != self.owner {
            return Err(invalid());
        }
        let mut seen = BTreeSet::new();
        for id in items.iter().flatten() {
            let point = self.points.get(*id).ok_or_else(invalid)?;
            if point.owner != owner || point.block != block || !point.complete || !seen.insert(*id)
            {
                return Err(invalid());
            }
            let valid = match source {
                Source::Block(_) => {
                    point.kind == PointKind::Stmt
                        && point
                            .site
                            .and_then(|site| self.sites.get(&site))
                            .is_some_and(|site| site.complete && site.point == Some(*id))
                }
                Source::Expr(parent) => {
                    point.parent == Some(parent)
                        && matches!(point.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                }
                Source::Stmt(parent) => {
                    point.parent == Some(parent) && point.kind == PointKind::Match
                }
            };
            if !valid {
                return Err(invalid());
            }
        }
        if let Some(prior) = self.sequences.get(&source) {
            return if prior.owner == owner && prior.items == items {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        let count = items
            .windows(2)
            .filter(|pair| pair[0].is_some() && pair[1].is_some())
            .count();
        if !self.edge_room(count) {
            return Err(budget());
        }
        let edges = items
            .windows(2)
            .filter_map(|pair| {
                Some(Edge::new(
                    Port::Normal(pair[0]?),
                    Port::Entry(pair[1]?),
                    Route::Next,
                ))
            })
            .collect();
        self.sequences.insert(
            source,
            Sequence {
                owner,
                items,
                edges,
            },
        );
        self.sequence_edges += count;
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod operands;
