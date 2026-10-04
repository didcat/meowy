use super::{walk::Walk, *};

mod locals;

pub(crate) const MAX_ENTRIES: usize = crate::flow::MAX_NODES;
pub(crate) const MAX_REPORT_ITEMS: usize = MAX_EDGES * 3 + MAX_ENTRIES * 2;

#[derive(Debug)]
pub(crate) struct Reports {
    pub(crate) index: ForwardIndex,
    pub(crate) locals: usize,
    pub(crate) eligible: std::collections::BTreeSet<crate::hir::LocalId>,
    pub(crate) initializers: super::initializers::Initializers,
    pub(crate) entries: BTreeMap<usize, (crate::hir::BlockId, Walk)>,
    pub(crate) effects: super::effects::Effects,
    pub(crate) parts: usize,
    pub(crate) blocks: super::blocks::Blocks,
    pub(crate) results: super::results::Results,
    pub(crate) consumers: super::consumers::Index,
    pub(crate) slot_uses: super::consumers::Uses,
    pub(crate) candidate_inputs: super::results::inputs::Inputs,
    pub(crate) calls: super::calls::CallGraph,
    pub(crate) groups: super::calls::Components,
    pub(crate) condensed: super::calls::Condensed,
    pub(crate) order: Vec<usize>,
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
        let locals = self.report_local_count(program, span)?;
        let mut reports = Reports {
            index: self.forward_index(span)?,
            locals,
            eligible: std::collections::BTreeSet::new(),
            initializers: BTreeMap::new(),
            entries: BTreeMap::new(),
            effects: BTreeMap::new(),
            parts: 0,
            blocks: BTreeMap::new(),
            results: BTreeMap::new(),
            consumers: BTreeMap::new(),
            slot_uses: BTreeMap::new(),
            candidate_inputs: BTreeMap::new(),
            calls: super::calls::CallGraph::default(),
            groups: super::calls::Components::default(),
            condensed: super::calls::Condensed::default(),
            order: Vec::new(),
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
        (reports.effects, reports.parts) = self.operation_effects_budgeted(&reports, span)?;
        reports.blocks = self.block_effects(&reports, span)?;
        (reports.results, reports.parts) = self.result_sources(&reports, span)?;
        reports.consumers = self.result_consumers(&reports, span)?;
        reports.eligible = self.eligible_locals(program, &reports, span)?;
        reports.initializers = self.binding_initializers(program, &reports, span)?;
        reports.slot_uses = self.slot_uses(&reports, span)?;
        (reports.candidate_inputs, reports.parts) = self.candidate_inputs(&reports, span)?;
        reports.calls = self.call_graph(&reports, span)?;
        reports.groups = reports.calls.components(&mut self.flow, span)?;
        reports.condensed = reports
            .calls
            .condense(&reports.groups, &mut self.flow, span)?;
        reports.order = reports.condensed.analysis_order(&mut self.flow, span)?;
        Ok(reports)
    }
}

#[cfg(test)]
mod tests;
