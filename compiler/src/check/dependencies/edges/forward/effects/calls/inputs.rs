use super::*;

#[derive(Debug)]
pub(in super::super::super) struct Inputs<'a> {
    pub(in super::super::super) call: &'a Invocation,
    pub(in super::super::super) pairs: std::iter::Zip<
        std::iter::Copied<std::slice::Iter<'a, hir::LocalId>>,
        std::iter::Copied<std::slice::Iter<'a, PointId>>,
    >,
}

impl Checker {
    pub(in super::super::super) fn checked_call_inputs<'a>(
        &'a mut self,
        reports: &'a Reports,
        site: hir::CallId,
        span: Span,
    ) -> Result<Inputs<'a>> {
        let budget = || Diagnostic::unsupported("proof call-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-effect identity mismatch", span);
        if !self.flow.spend(
            self.invocations.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.proofs.calls.len().checked_ilog2().unwrap_or(0) as usize
                + 6,
        ) {
            return Err(budget());
        }
        let call = self.invocations.get(&site).ok_or_else(invalid)?;
        if call.args.len() > crate::check::dependencies::sequences::MAX_ITEMS
            || !self.flow.spend(
                call.args.len() * (call.args.len().checked_ilog2().unwrap_or(0) as usize + 4)
                    + reports.entries.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + 10,
            )
        {
            return Err(budget());
        }
        let owner = call.function.checked_add(1).ok_or_else(invalid)?;
        let (body, _) = reports.entries.get(&owner).ok_or_else(invalid)?;
        let (caller, _) = reports.entries.get(&call.owner).ok_or_else(invalid)?;
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
            || !self
                .bodies
                .get(caller)
                .is_some_and(|body| body.owner == call.owner && body.parent.is_none())
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
        let (function, count, shape) = (call.function, call.args.len(), call.shape);
        let header = self.checked_function_signature(reports, function, span)?;
        if header.params.len() != count || header.shape != shape {
            return Err(invalid());
        }
        if !self.flow.spend(count + 1) {
            return Err(budget());
        }
        let call = self.invocations.get(&site).expect("validated call");
        Ok(Inputs {
            call,
            pairs: header.params.iter().copied().zip(call.args.iter().copied()),
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
