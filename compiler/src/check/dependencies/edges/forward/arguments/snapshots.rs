use super::program::Calls;
use super::*;
use crate::borrow::{Facts, State};
use crate::check::dependencies::Invocation;
use crate::hir::{ExprKind, Program};

mod payload;

pub(in super::super) struct Snapshots<'a> {
    pub(super) calls: Calls<'a>,
    pub(super) program: &'a Program,
    pub(super) access: super::mutations::Accesses,
    pub(super) facts: &'a Facts,
    pub(super) reports: &'a Reports,
}

pub(in super::super) type Input<'a> = (hir::LocalId, PointId, Option<(&'a State, bool)>);

impl Checker {
    pub(in super::super) fn call_snapshots<'a>(
        &mut self,
        program: &'a Program,
        reports: &'a Reports,
        facts: &'a Facts,
        span: Span,
        parts: usize,
    ) -> Result<(Snapshots<'a>, usize)> {
        if facts.call_inputs.len() > crate::borrow::MAX_FACT_ORIGINS
            || !self.flow.spend(facts.call_inputs.len() + 1)
        {
            return Err(payload::budget(span));
        }
        let calls = self.program_calls(program, reports, span, parts)?;
        let parts = parts
            .min(MAX_EDGES)
            .checked_sub(calls.len())
            .ok_or_else(|| payload::budget(span))?;
        let (access, mut parts) = self.declared_access(program, reports, span, parts)?;
        let view = Snapshots {
            calls,
            program,
            access,
            facts,
            reports,
        };
        for &site in facts.call_inputs.keys() {
            let (_, cost) = self.checked_borrow_snapshot(&view, site, span, parts)?;
            parts -= cost;
        }
        Ok((view, parts))
    }

    pub(super) fn checked_borrow_snapshot<'a>(
        &mut self,
        view: &Snapshots<'a>,
        site: hir::CallId,
        span: Span,
        parts: usize,
    ) -> Result<(Option<&'a crate::borrow::calls::Inputs>, usize)> {
        if !self.flow.spend(
            view.calls.len().checked_ilog2().unwrap_or(0) as usize
                + view.facts.call_inputs.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.calls.len().checked_ilog2().unwrap_or(0) as usize
                + 4,
        ) {
            return Err(payload::budget(span));
        }
        let (_, body, expr) = view
            .calls
            .get(&site)
            .ok_or_else(|| payload::invalid(span))?;
        let ExprKind::Call { id, args, .. } = &expr.kind else {
            return Err(payload::invalid(span));
        };
        let Some(snapshot) = view.facts.call_inputs.get(&site) else {
            return Ok((None, 0));
        };
        let header = snapshot.header;
        if header.site != site
            || header.function != *id
            || header.body != *body
            || header.count != args.len()
            || self.proofs.calls.get(&site) != Some(&header.entered)
        {
            return Err(payload::invalid(span));
        }
        let cost = payload::validate(snapshot, &mut self.flow, span, parts)?;
        Ok((Some(snapshot), cost))
    }

    pub(in super::super) fn call_borrow_inputs<'a>(
        &'a mut self,
        view: &'a Snapshots<'a>,
        site: hir::CallId,
        span: Span,
    ) -> Result<(
        &'a Invocation,
        impl ExactSizeIterator<Item = Input<'a>> + 'a,
    )> {
        let (snapshot, _) = self.checked_borrow_snapshot(view, site, span, MAX_EDGES)?;
        let (owner, _, expr) = view
            .calls
            .get(&site)
            .ok_or_else(|| payload::invalid(span))?;
        let ExprKind::Call { id, args, .. } = &expr.kind else {
            return Err(payload::invalid(span));
        };
        if !self.flow.spend(args.len() + 1) {
            return Err(payload::budget(span));
        }
        let inputs = self.checked_call_inputs(view.reports, site, span)?;
        if inputs.call.owner != *owner
            || inputs.call.function != *id
            || inputs.call.args.len() != args.len()
            || inputs.call.shape != Invocation::result_shape(&expr.ty)
        {
            return Err(payload::invalid(span));
        }
        Ok((
            inputs.call,
            inputs
                .pairs
                .enumerate()
                .map(move |(position, (param, point))| {
                    let state = snapshot.and_then(|snapshot| {
                        snapshot
                            .args
                            .get(position)
                            .map(|(_, state)| (state, position < snapshot.normal))
                    });
                    (param, point, state)
                }),
        ))
    }

    pub(crate) fn qualify_borrow_calls(
        &mut self,
        program: &Program,
        reports: &Reports,
        facts: &Facts,
        span: Span,
    ) -> Result<super::Mutations> {
        self.scalar_call_mutations(program, reports, facts, span, MAX_EDGES)
            .map(|(mutations, _)| mutations)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
