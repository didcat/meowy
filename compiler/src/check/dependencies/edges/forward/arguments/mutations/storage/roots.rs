use super::*;
use crate::borrow::Origin;
use crate::check::dependencies::references::MAX_ROOTS;
use crate::flow::{FALSE, Guard};

pub(in super::super) type Roots = BTreeMap<hir::LocalId, Guard>;

impl Checker {
    pub(in super::super) fn mutation_fact_roots(
        &mut self,
        reports: &Reports,
        owner: usize,
        origins: &[(&Origin, PointId)],
        span: Span,
        parts: usize,
    ) -> Result<(Roots, usize)> {
        let budget = || Diagnostic::unsupported("proof call-fact-root budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-fact-root identity mismatch", span);
        if origins.len() > crate::borrow_value::MAX_PARTS || !self.flow.spend(origins.len() + 1) {
            return Err(budget());
        }
        let mut roots = Roots::new();
        let mut parts = parts.min(MAX_EDGES);
        for &(origin, point) in origins {
            if !origin.component.is_empty() || origin.guard >> 1 >= self.flow.nodes.len() {
                return Err(invalid());
            }
            let ((found, source), _) = self.mutation_storage_key(point, span)?;
            if !self.flow.spend(
                origin.weight()
                    + self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.proofs.bindings.len().checked_ilog2().unwrap_or(0) as usize
                    + 5,
            ) {
                return Err(budget());
            }
            let op = &self.place_borrows[&point];
            let root = self.origin_id(op.place.root);
            if found != owner
                || source != origin.source
                || op.mode != ReferenceMode::Exclusive
                || root != op.storage
                || self.origin_id(root) != root
                || root >= reports.locals
                || !self.proofs.bindings.contains_key(&root)
            {
                return Err(invalid());
            }
            self.merge_mutation_root(&mut roots, root, origin.guard, span, &mut parts)?;
        }
        Ok((roots, parts))
    }

    pub(in super::super) fn merge_mutation_root(
        &mut self,
        roots: &mut Roots,
        root: hir::LocalId,
        guard: Guard,
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof call-fact-root budget exhausted", span);
        if !self
            .flow
            .spend(roots.len().checked_ilog2().unwrap_or(0) as usize * 2 + 2)
        {
            return Err(budget());
        }
        let prior = roots.get(&root).copied();
        if prior.is_none() && (roots.len() >= MAX_ROOTS || *parts == 0) {
            return Err(budget());
        }
        let merged = self.flow.or(prior.unwrap_or(FALSE), guard);
        if self.flow.exceeded() {
            return Err(budget());
        }
        *parts -= usize::from(prior.is_none());
        roots.insert(root, merged);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
