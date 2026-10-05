use super::*;
use crate::check::dependencies::edges::forward::effects::Effect;

pub(super) type Index = BTreeMap<hir::BlockId, (usize, PointId)>;

impl Checker {
    pub(super) fn dispatch_result_index(
        &mut self,
        reports: &Reports,
        parts: &mut usize,
        limit: usize,
        span: Span,
    ) -> Result<Index> {
        let budget = || Diagnostic::unsupported("proof dispatch-source budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof dispatch-source identity mismatch", span);
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget());
        }
        let mut room = *parts;
        let mut index = Index::new();
        for (&id, (owner, effect)) in &reports.effects {
            if !matches!(effect, Effect::Dispatch(_)) {
                continue;
            }
            let Some(block) = self.dispatch_result_body(reports, id, *owner, span)? else {
                continue;
            };
            if !self.flow.spend(
                index.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + reports.blocks.len().checked_ilog2().unwrap_or(0) as usize
                    + 3,
            ) {
                return Err(budget());
            }
            if reports.blocks.contains_key(&block) || index.contains_key(&block) {
                return Err(invalid());
            }
            if index.len() >= limit.min(MAX_EDGES) {
                return Err(budget());
            }
            room = room.checked_sub(1).ok_or_else(budget)?;
            index.insert(block, (*owner, id));
        }
        *parts = room;
        Ok(index)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
