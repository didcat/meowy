use super::*;
use crate::check::dependencies::{Invocation, bodies::completion::Shape};
use crate::hir;

mod stages;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Operation,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) site: hir::CallId,
    pub(super) function: hir::FunctionId,
    pub(super) count: usize,
    pub(super) shape: Shape,
    pub(super) may_return: bool,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

impl Checker {
    pub(super) fn call_effect_index(
        &mut self,
        span: Span,
    ) -> Result<BTreeMap<PointId, hir::CallId>> {
        self.call_effect_index_limited(span, MAX_EDGES)
    }

    pub(super) fn call_effect_stage(
        &mut self,
        reports: &Reports,
        calls: &BTreeMap<PointId, hir::CallId>,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let (id, kind) = match port {
            Port::Operation(id) => (id, Kind::Operation),
            Port::Normal(id) => (id, Kind::Result),
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(calls.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(Diagnostic::unsupported(
                "proof call-effect budget exhausted",
                span,
            ));
        }
        let Some(&site) = calls.get(&id) else {
            return Ok(None);
        };
        let call = self.checked_call_effect(reports, site, span)?;
        if call.point != id || call.owner != owner || (kind == Kind::Result && !call.may_return) {
            return Err(Diagnostic::unsupported(
                "proof call-effect identity mismatch",
                span,
            ));
        }
        Ok(Some(Stage {
            point: id,
            owner,
            site,
            function: call.function,
            count: call.args.len(),
            shape: call.shape,
            may_return: call.may_return,
            control: call.control,
            kind,
        }))
    }

    pub(super) fn checked_call_effect(
        &mut self,
        reports: &Reports,
        site: hir::CallId,
        span: Span,
    ) -> Result<&Invocation> {
        let budget = || Diagnostic::unsupported("proof call-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-effect identity mismatch", span);
        if !self.flow.spend(
            self.invocations.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.calls.len().checked_ilog2().unwrap_or(0) as usize
                + 6,
        ) {
            return Err(budget());
        }
        let call = self.invocations.get(&site).ok_or_else(invalid)?;
        if call.args.len() > crate::check::dependencies::sequences::MAX_ITEMS
            || !self.flow.spend(
                call.args.len() * (call.args.len().checked_ilog2().unwrap_or(0) as usize + 4)
                    + reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + 8,
            )
        {
            return Err(budget());
        }
        let owner = call.function.checked_add(1).ok_or_else(invalid)?;
        let (body, _) = reports.entries.get(&owner).ok_or_else(invalid)?;
        let point = self.points.get(call.point).ok_or_else(invalid)?;
        if !call.valid_result()
            || call.site != site
            || site >= self.calls
            || !self.proofs.calls.contains_key(&site)
            || reports.index.operations.get(&call.point) != Some(&call.owner)
            || !point.complete
            || point.kind != PointKind::Expr
            || point.owner != call.owner
            || point.span != call.span
            || !self
                .bodies
                .get(body)
                .is_some_and(|body| body.owner == owner && body.parent.is_none())
            || call.edges.len() != call.args.len() + 1 + usize::from(call.may_return)
        {
            return Err(invalid());
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut from = Port::Entry(call.point);
        for (index, arg) in call.args.iter().enumerate() {
            if !seen.insert(*arg)
                || !self.points.get(*arg).is_some_and(|arg| {
                    arg.complete
                        && arg.parent == Some(call.point)
                        && arg.owner == call.owner
                        && arg.block == point.block
                        && matches!(arg.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                })
                || call.edges[index] != Edge::new(from, Port::Entry(*arg), Route::Next)
            {
                return Err(invalid());
            }
            from = Port::Normal(*arg);
        }
        let operation = Port::Operation(call.point);
        if call.edges[call.args.len()] != Edge::new(from, operation, Route::Next)
            || call.may_return
                && call.edges.last()
                    != Some(&Edge::new(
                        operation,
                        Port::Normal(call.point),
                        Route::Returned,
                    ))
        {
            return Err(invalid());
        }
        Ok(call)
    }

    pub(self) fn call_effect_index_limited(
        &mut self,
        span: Span,
        limit: usize,
    ) -> Result<BTreeMap<PointId, hir::CallId>> {
        let budget = || Diagnostic::unsupported("proof call-effect index budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-effect identity mismatch", span);
        if self.invocations.len() > limit || !self.flow.spend(self.invocations.len() + 1) {
            return Err(budget());
        }
        let mut calls = BTreeMap::new();
        for (&site, call) in &self.invocations {
            if !self.flow.spend(
                calls.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.proofs.calls.len().checked_ilog2().unwrap_or(0) as usize
                    + 9,
            ) {
                return Err(budget());
            }
            if !call.valid_result()
                || site != call.site
                || site >= self.calls
                || !self.proofs.calls.contains_key(&site)
                || !self.points.get(call.point).is_some_and(|point| {
                    point.kind == PointKind::Expr
                        && point.complete
                        && point.owner == call.owner
                        && point.span == call.span
                })
                || calls.insert(call.point, site).is_some()
            {
                return Err(invalid());
            }
        }
        Ok(calls)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod reports;

#[cfg(test)]
mod observations;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod shapes;
