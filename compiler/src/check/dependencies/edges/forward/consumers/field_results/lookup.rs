use super::*;

pub(in super::super::super) mod narrowing;

pub(in super::super::super) struct Lookup<'a> {
    pub(in super::super::super) reports: &'a Reports,
    pub(in super::super::super) parts: usize,
    pub(super) roots: Option<BTreeMap<Slot, usize>>,
    pub(super) checked: BTreeMap<PointId, Slot>,
}

impl<'a> Lookup<'a> {
    pub(in super::super::super) fn new(reports: &'a Reports, parts: usize) -> Self {
        Self {
            reports,
            parts: parts.min(MAX_EDGES),
            roots: None,
            checked: BTreeMap::new(),
        }
    }
}

impl Checker {
    pub(in super::super::super) fn field_result_source(
        &mut self,
        ctx: &mut Lookup<'_>,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<Slot>> {
        let budget = || Diagnostic::unsupported("proof field-result lookup budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof field-result lookup identity mismatch", span);
        let reports = ctx.reports;
        if !self.flow.spend(
            reports.field_results.len().checked_ilog2().unwrap_or(0) as usize
                + reports.effects.len().checked_ilog2().unwrap_or(0) as usize
                + ctx.checked.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 5,
        ) {
            return Err(budget());
        }
        let Some(&(source_owner, slot)) = reports.field_results.get(&Port::Normal(id)) else {
            return Ok(None);
        };
        if source_owner != owner {
            return Err(invalid());
        }
        if let Some(&prior) = ctx.checked.get(&id) {
            return if prior == slot {
                Ok(Some(slot))
            } else {
                Err(invalid())
            };
        }
        let (reported_owner, effect) = reports.effects.get(&id).ok_or_else(invalid)?;
        if *reported_owner != owner
            || self.field_result_slot(reports, id, owner, effect, span)? != Some(slot)
        {
            return Err(invalid());
        }
        if ctx.roots.is_none() {
            let (roots, parts) = self.field_result_roots(reports, span, ctx.parts)?;
            ctx.roots = Some(roots);
            ctx.parts = parts;
        }
        let roots = ctx.roots.as_ref().unwrap();
        if !self
            .flow
            .spend(roots.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        if roots.get(&slot) != Some(&owner) {
            return Err(invalid());
        }
        ctx.parts = ctx.parts.checked_sub(1).ok_or_else(budget)?;
        ctx.checked.insert(id, slot);
        Ok(Some(slot))
    }
}

#[cfg(test)]
mod tests;
