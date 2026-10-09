use super::{entries::Reports, *};
use crate::{
    check::dependencies::{
        ScalarKind,
        bodies::{Completion, Fact},
    },
    hir,
};

mod shared;

pub(crate) type Receivers = BTreeMap<hir::LocalId, (usize, PointId, Option<ScalarKind>)>;

impl Checker {
    pub(super) fn receiver_index(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        span: Span,
        limit: usize,
        mut parts: usize,
    ) -> Result<(Receivers, usize)> {
        let budget = || Diagnostic::unsupported("proof receiver-index budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof receiver-index identity mismatch", span);
        if self.dispatch_ops.len() > limit.min(MAX_EDGES)
            || !self.flow.spend(self.dispatch_ops.len() + 1)
        {
            return Err(budget());
        }
        let mut index = Receivers::new();
        for (&id, op) in &self.dispatch_ops {
            let work = [
                self.bodies.len(),
                self.bodies.len(),
                self.proofs.dispatches.len(),
                self.proofs.receivers.len(),
                self.proofs.aliases.len(),
                self.proofs.temporaries.len(),
                reports.entries.len(),
                index.len(),
                index.len(),
            ]
            .into_iter()
            .fold(35, |work, len| {
                work + len.checked_ilog2().unwrap_or(0) as usize
            });
            if !self.flow.spend(work) {
                return Err(budget());
            }
            let point = self.points.get(id).ok_or_else(invalid)?;
            let body = self.bodies.get(&op.block).ok_or_else(invalid)?;
            let ty = program.locals.get(op.local).ok_or_else(invalid)?;
            let shape = Completion::of(ty);
            if op.local >= reports.locals
                || !reports.entries.contains_key(&op.owner)
                || point.owner != op.owner
                || !point.complete
                || point.kind != PointKind::Expr
                || point.span != op.span
                || point.span.start > point.span.end
                || body.owner != op.owner
                || body.parent != Some(id)
                || body.span.start > body.span.end
                || body.span.start < point.span.start
                || body.span.end > point.span.end
                || !point
                    .block
                    .filter(|block| *block != op.block)
                    .and_then(|block| self.bodies.get(&block))
                    .is_some_and(|body| body.owner == op.owner)
                || op.input == id
                || !self.points.get(op.input).is_some_and(|input| {
                    input.complete
                        && input.owner == op.owner
                        && input.parent == Some(id)
                        && input.block == point.block
                        && input.span.start >= point.span.start
                        && input.span.start <= input.span.end
                        && input.span.end <= body.span.start
                        && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                })
                || shape.normal != op.input_normal
                || shape.result != op.receiver
                || !self.proofs.dispatches.contains(&op.block)
                || !self.proofs.receivers.contains(&op.local)
                || self.proofs.aliases.contains_key(&op.local)
                || self.proofs.temporaries.contains_key(&op.local)
                || !matches!(body.facts.first(), Some((Fact::Bind(local), _)) if *local == op.local)
                || body.links.first() != Some(&None)
                || body.sources.first() != Some(&None)
                || body.storage.first() != Some(&Some(op.local))
                || index.contains_key(&op.local)
            {
                return Err(invalid());
            }
            let mut primary = Self::shared_receiver_type(ty, &mut self.flow, span)?;
            if primary.is_some() {
                if !self.flow.spend(
                    self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                        + self.proofs.fields.len().checked_ilog2().unwrap_or(0) as usize
                        + 2,
                ) {
                    return Err(budget());
                }
                if self.proofs.variable(op.local) {
                    primary = None;
                }
            }
            if primary != op.shared_primary {
                return Err(invalid());
            }
            parts = parts.checked_sub(1).ok_or_else(budget)?;
            index.insert(op.local, (op.owner, id, primary));
        }
        Ok((index, parts))
    }
}

#[cfg(test)]
mod tests;
