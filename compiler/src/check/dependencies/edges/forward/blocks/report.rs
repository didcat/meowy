use super::*;
use crate::{check::dependencies::bodies::Completion, hir};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) parent: Option<hir::PointId>,
    pub(crate) span: Span,
    pub(crate) completion: Completion,
    pub(crate) normal: bool,
    pub(crate) result: bool,
}

pub(crate) type Blocks = BTreeMap<hir::BlockId, (usize, Observed)>;

impl Checker {
    pub(super) fn record_block_effect(
        &mut self,
        owner: usize,
        port: Port,
        blocks: &mut Blocks,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof block-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof block-effect identity mismatch", span);
        let id = match port {
            Port::BlockNormal(id) | Port::BlockResult(id) => id,
            _ => return Err(invalid()),
        };
        if !self.flow.spend(
            self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + blocks.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 10,
        ) {
            return Err(budget());
        }
        let body = self.bodies.get(&id).ok_or_else(invalid)?;
        let mut observed = Observed {
            parent: body.parent,
            span: body.span,
            completion: body.completion,
            normal: false,
            result: false,
        };
        if let Some((prior_owner, prior)) = blocks.get(&id) {
            if *prior_owner != owner
                || prior.parent != observed.parent
                || prior.span != observed.span
                || prior.completion != observed.completion
            {
                return Err(invalid());
            }
            observed.normal = prior.normal;
            observed.result = prior.result;
        } else if blocks.len() >= limit {
            return Err(budget());
        }
        match port {
            Port::BlockNormal(_) => observed.normal = true,
            Port::BlockResult(_) => observed.result = true,
            _ => unreachable!(),
        }
        blocks.insert(id, (owner, observed));
        Ok(())
    }

    pub(in super::super) fn block_effects(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<Blocks> {
        self.block_effects_limited(reports, span, MAX_EDGES)
    }

    pub(super) fn block_effects_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
    ) -> Result<Blocks> {
        let budget = || Diagnostic::unsupported("proof block-effect budget exhausted", span);
        let limit = limit
            .checked_sub(reports.effects.len())
            .ok_or_else(budget)?;
        if !self.flow.spend(reports.entries.len() + 1) {
            return Err(budget());
        }
        let mut blocks = Blocks::new();
        for (&owner, (_, walk)) in &reports.entries {
            if !self.flow.spend(walk.ports.len() + 1) {
                return Err(budget());
            }
            for &port in &walk.ports {
                if self.validate_block_effect(reports, owner, port, span)? {
                    self.record_block_effect(owner, port, &mut blocks, limit, span)?;
                }
            }
        }
        Ok(blocks)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
