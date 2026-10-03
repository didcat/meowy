use super::PointKind;
use crate::{
    ast::Span,
    check::{Checker, Result},
    diagnostic::Diagnostic,
    hir,
};

pub(crate) const MAX_GROUPS: usize = super::sequences::MAX_ITEMS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Group {
    pub(crate) owner: usize,
    pub(crate) input: hir::PointId,
    pub(crate) block: Option<hir::BlockId>,
    pub(crate) span: Span,
}

impl Checker {
    pub(crate) fn group_region(
        &mut self,
        id: hir::PointId,
        input: hir::PointId,
        span: Span,
    ) -> Result<()> {
        if self.required {
            return self.region_edges(id, input, span);
        }
        let budget = || Diagnostic::unsupported("proof group-input budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof group-input identity mismatch", span);
        if !self
            .flow
            .spend(self.group_inputs.len().checked_ilog2().unwrap_or(0) as usize * 2 + 4)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if point.kind != PointKind::Expr
            || point.owner != self.owner
            || point.span != span
            || (!point.complete && self.point != Some(id))
            || id == input
            || !self.points.get(input).is_some_and(|child| {
                child.parent == Some(id)
                    && child.owner == point.owner
                    && child.block == point.block
                    && child.complete
                    && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
        {
            return Err(invalid());
        }
        let group = Group {
            owner: self.owner,
            input,
            block: point.block,
            span,
        };
        if let Some(prior) = self.group_inputs.get(&id) {
            if *prior != group {
                return Err(invalid());
            }
        } else if self.group_inputs.len() >= MAX_GROUPS {
            return Err(budget());
        }
        self.region_edges(id, input, span)?;
        self.group_inputs.insert(id, group);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
