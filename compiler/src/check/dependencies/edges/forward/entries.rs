use super::{walk::Walk, *};

pub(crate) const MAX_ENTRIES: usize = crate::flow::MAX_NODES;
pub(crate) const MAX_REPORT_ITEMS: usize = MAX_EDGES * 3 + MAX_ENTRIES * 2;

#[derive(Debug)]
pub(crate) struct Reports {
    pub(crate) index: ForwardIndex,
    pub(crate) entries: BTreeMap<usize, (crate::hir::BlockId, Walk)>,
    pub(self) items: usize,
}

impl Checker {
    pub(crate) fn entry_reports(
        &mut self,
        program: &crate::hir::Program,
        span: Span,
    ) -> Result<Reports> {
        self.entry_reports_limited(program, span, MAX_ENTRIES, MAX_REPORT_ITEMS)
    }

    pub(self) fn entry_reports_limited(
        &mut self,
        program: &crate::hir::Program,
        span: Span,
        roots: usize,
        items: usize,
    ) -> Result<Reports> {
        let budget = || Diagnostic::unsupported("proof entry-report budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof entry-report owner mismatch", span);
        if program.functions.len() >= roots || !self.flow.spend(1) {
            return Err(budget());
        }
        let mut reports = Reports {
            index: self.forward_index(span)?,
            entries: BTreeMap::new(),
            items: 0,
        };
        let entries = std::iter::once((None, program.body.id)).chain(
            program
                .functions
                .iter()
                .map(|function| (Some(function.id), function.body.id)),
        );
        for (function, block) in entries {
            if !self
                .flow
                .spend(reports.entries.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3)
            {
                return Err(budget());
            }
            let owner = match function {
                Some(id) => id.checked_add(1).ok_or_else(invalid)?,
                None => 0,
            };
            let start = Port::BlockEntry(block);
            if reports.entries.contains_key(&owner) || self.port_owner(start, span)? != owner {
                return Err(invalid());
            }
            let walk =
                reports
                    .index
                    .walk_limited(start, &mut self.flow, span, items - reports.items)?;
            reports.items += walk.len();
            reports.entries.insert(owner, (block, walk));
        }
        Ok(reports)
    }
}

#[cfg(test)]
mod tests;
