use super::entries::Reports;
use super::*;
use crate::hir;

#[cfg(test)]
mod copies;
mod declarations;
mod definitions;
mod domain;
mod events;
mod graph;
mod guards;
#[cfg(test)]
mod literals;
mod observations;
mod ports;
mod schedule;
#[cfg(test)]
mod sources;
mod state;
mod versions;

impl Checker {
    pub(crate) fn qualify_scalar_facts(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        calls: &super::arguments::Mutations,
        span: Span,
    ) -> Result<()> {
        let (versions, events, entries, parts) =
            self.scalar_state_context(program, reports, span, MAX_EDGES)?;
        let (ports, parts) = self.scalar_ports(reports, &entries, span, parts)?;
        let (graph, parts) = self.scalar_graph(reports, ports, span, parts)?;
        let (orders, parts) = self.scalar_schedule(&graph, &entries, span, parts)?;
        let (definitions, parts) =
            self.scalar_definitions(reports, &versions, &events, &graph, span, parts)?;
        let (observations, parts) =
            self.scalar_observations(reports, &graph, &definitions, span, parts)?;
        let (observations, parts) =
            self.scalar_read_observations(program, reports, &graph, observations, span, parts)?;
        let (observations, parts) =
            self.scalar_borrow_observations(program, reports, &graph, observations, span, parts)?;
        let (observations, parts) =
            self.scalar_call_observations(reports, &graph, calls, observations, span, parts)?;
        let (observations, parts) =
            self.scalar_call_result_observations(reports, &graph, observations, span, parts)?;
        let (observations, parts) =
            self.scalar_scope_observations(reports, &graph, observations, span, parts)?;
        let (observations, parts) =
            self.scalar_emission_observations(reports, &graph, observations, span, parts)?;
        self.scalar_snapshots(&graph, &orders, &entries, &observations, span, parts)?;
        Ok(())
    }

    pub(self) fn scalar_state_context<'a>(
        &mut self,
        program: &'a hir::Program,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(
        versions::Versions<'a>,
        events::Events<'a>,
        state::Entries,
        usize,
    )> {
        let (declared, parts) = self.scalar_declared_seeds(program, reports, span, parts)?;
        let (versions, parts) =
            self.scalar_version_seeds(program, reports, &declared, span, parts)?;
        let (events, parts) = self.scalar_storage_events(program, reports, span, parts)?;
        let (entries, parts) =
            self.scalar_entry_states(reports, &versions, &events, span, parts)?;
        Ok((versions, events, entries, parts))
    }

    #[cfg(test)]
    pub(self) fn scalar_event_context<'a>(
        &mut self,
        program: &'a hir::Program,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(versions::Versions<'a>, events::Events<'a>, usize)> {
        let (versions, parts) = self.scalar_copy_context(program, reports, span, parts)?;
        let (events, parts) = self.scalar_storage_events(program, reports, span, parts)?;
        Ok((versions, events, parts))
    }

    #[cfg(test)]
    pub(self) fn scalar_copy_context<'a>(
        &mut self,
        program: &'a hir::Program,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(versions::Versions<'a>, usize)> {
        let (literals, declared, parts) =
            self.scalar_seed_context(program, reports, span, parts)?;
        let (versions, parts) =
            self.scalar_version_seeds(program, reports, &declared, span, parts)?;
        let (inputs, parts) = self.scalar_copy_inputs(reports, &versions, span, parts)?;
        self.scalar_copies(versions, &inputs, &literals, span, parts)
    }

    #[cfg(test)]
    pub(self) fn scalar_seed_context<'a>(
        &mut self,
        program: &'a hir::Program,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(
        literals::Literals<'a>,
        declarations::Declarations<'a>,
        usize,
    )> {
        let (declared, parts) = self.scalar_declared_seeds(program, reports, span, parts)?;
        let (literals, parts) = self.scalar_literal_seeds(reports, span, parts)?;
        Ok((literals, declared, parts))
    }
}

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod associations;
