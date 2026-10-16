use super::*;
use crate::check::dependencies::{Invocation, bodies::completion::Shape};
use crate::hir;

mod inputs;
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
    pub(in super::super) fn call_effect_index(
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
        let inputs = self.checked_call_inputs(reports, site, span)?;
        let call = inputs.call;
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
            count: inputs.pairs.len(),
            shape: call.shape,
            may_return: call.may_return,
            control: call.control,
            kind,
        }))
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

#[cfg(test)]
mod parameters;

#[cfg(test)]
mod arguments;
