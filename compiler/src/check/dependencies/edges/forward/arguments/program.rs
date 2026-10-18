use super::*;
use crate::check::dependencies::Invocation;
use crate::hir::{Expr, ExprKind, Program};

#[cfg(test)]
pub(super) mod blocks;
mod scan;

pub(in super::super) type Calls<'a> = BTreeMap<hir::CallId, (usize, hir::BlockId, &'a Expr)>;

impl Checker {
    pub(in super::super) fn program_calls<'a>(
        &mut self,
        program: &'a Program,
        reports: &Reports,
        span: Span,
        limit: usize,
    ) -> Result<Calls<'a>> {
        let budget = || Diagnostic::unsupported("proof HIR-call index budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof HIR-call identity mismatch", span);
        self.program_signatures(program, reports, span)?;
        let mut calls = Calls::new();
        let roots = std::iter::once((0, &program.body)).chain(
            program
                .functions
                .iter()
                .map(|function| (function.id + 1, &function.body)),
        );
        for (owner, body) in roots {
            if !self.flow.spend(1) {
                return Err(budget());
            }
            let mut scan = scan::Scan {
                pending: vec![(scan::Item::Statements(&body.stmts), 0)],
            };
            while let Some(expr) = scan.next(&mut self.flow, span)? {
                let ExprKind::Call { id, site, args } = &expr.kind else {
                    unreachable!()
                };
                if calls.len() >= limit.min(MAX_EDGES)
                    || !self
                        .flow
                        .spend(calls.len().checked_ilog2().unwrap_or(0) as usize * 2 + 1)
                {
                    return Err(budget());
                }
                let inputs = self.checked_call_inputs(reports, *site, span)?;
                if calls.contains_key(site)
                    || inputs.call.owner != owner
                    || inputs.call.function != *id
                    || inputs.call.args.len() != args.len()
                    || inputs.call.shape != Invocation::result_shape(&expr.ty)
                {
                    return Err(invalid());
                }
                calls.insert(*site, (owner, body.id, expr));
            }
        }
        if calls.len() != self.invocations.len() {
            return Err(invalid());
        }
        Ok(calls)
    }
}

#[cfg(test)]
mod tests;
