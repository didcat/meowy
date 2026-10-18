use super::*;
use crate::hir::Program;

mod access;
#[cfg(test)]
mod storage;

pub(in super::super) use access::Access;
pub(in super::super) type Accesses = BTreeMap<hir::LocalId, Access>;

pub(in super::super) type Input<'a> = (
    hir::LocalId,
    PointId,
    Access,
    Option<(&'a crate::borrow::State, bool)>,
);

impl Checker {
    pub(in super::super) fn declared_access(
        &mut self,
        program: &Program,
        reports: &Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Accesses, usize)> {
        let budget = || Diagnostic::unsupported("proof declared-access budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof declared-access identity mismatch", span);
        let params = &self.program_signatures(program, reports, span)?.params;
        let count = params.len();
        let parts = parts.min(MAX_EDGES).checked_sub(count).ok_or_else(budget)?;
        if !self
            .flow
            .spend(count.saturating_mul(count.checked_ilog2().unwrap_or(0) as usize + 2) + 1)
        {
            return Err(budget());
        }
        let mut access = Accesses::new();
        for &param in params.keys() {
            let ty = program.locals.get(param).ok_or_else(invalid)?;
            access.insert(param, access::classify(ty, &mut self.flow, span)?);
        }
        Ok((access, parts))
    }

    pub(in super::super) fn call_mutation_inputs<'a>(
        &'a mut self,
        view: &'a super::snapshots::Snapshots<'a>,
        site: hir::CallId,
        span: Span,
    ) -> Result<(
        &'a crate::check::dependencies::Invocation,
        impl ExactSizeIterator<Item = Input<'a>> + 'a,
    )> {
        let budget = || Diagnostic::unsupported("proof declared-access budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof declared-access identity mismatch", span);
        if view.access.len() > MAX_EDGES
            || !self
                .flow
                .spend(view.calls.len().checked_ilog2().unwrap_or(0) as usize + 2)
        {
            return Err(budget());
        }
        if view.access.len() != view.reports.signatures.params.len() {
            return Err(invalid());
        }
        let (_, _, expr) = view.calls.get(&site).ok_or_else(invalid)?;
        let hir::ExprKind::Call { id, .. } = &expr.kind else {
            return Err(invalid());
        };
        let header = self.checked_function_signature(view.reports, *id, span)?;
        if !self.flow.spend(
            header
                .params
                .len()
                .saturating_mul(view.access.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3)
                + 1,
        ) {
            return Err(budget());
        }
        for param in &header.params {
            let ty = view.program.locals.get(*param).ok_or_else(invalid)?;
            let expected = access::classify(ty, &mut self.flow, span)?;
            if view.access.get(param) != Some(&expected) {
                return Err(invalid());
            }
        }
        let (call, inputs) = self.call_borrow_inputs(view, site, span)?;
        Ok((
            call,
            inputs.map(move |(param, point, state)| (param, point, view.access[&param], state)),
        ))
    }
}

#[cfg(test)]
mod tests;
