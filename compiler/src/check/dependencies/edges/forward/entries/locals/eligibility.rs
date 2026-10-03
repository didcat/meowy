use super::*;
use crate::hir::{LocalId, Program, Type};
use std::collections::BTreeSet;

pub(super) const MAX_TYPES: usize = crate::check::dependencies::sequences::MAX_ITEMS;

impl Checker {
    pub(in super::super) fn eligible_locals(
        &mut self,
        program: &Program,
        reports: &Reports,
        span: Span,
    ) -> Result<BTreeSet<LocalId>> {
        self.eligible_locals_limited(program, reports, span, MAX_EDGES)
    }

    pub(super) fn eligible_locals_limited(
        &mut self,
        program: &Program,
        reports: &Reports,
        span: Span,
        limit: usize,
    ) -> Result<BTreeSet<LocalId>> {
        let budget = || Diagnostic::unsupported("proof local-eligibility budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof local-eligibility identity mismatch", span);
        if program.locals.len() > MAX_EDGES || !self.flow.spend(program.locals.len() + 1) {
            return Err(budget());
        }
        if reports.locals != program.locals.len() || self.proofs.bindings.len() != reports.locals {
            return Err(invalid());
        }
        let limit = limit
            .min(MAX_EDGES)
            .checked_sub(reports.effects.len())
            .and_then(|room| room.checked_sub(reports.blocks.len()))
            .and_then(|room| room.checked_sub(reports.results.len()))
            .and_then(|room| room.checked_sub(reports.consumers.len()))
            .and_then(|room| room.checked_sub(reports.slot_uses.len()))
            .ok_or_else(budget)?;
        let mut eligible = BTreeSet::new();
        for (id, ty) in program.locals.iter().enumerate() {
            if !self.flow.spend(
                self.proofs.bindings.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.fields.len().checked_ilog2().unwrap_or(0) as usize
                    + 4,
            ) {
                return Err(budget());
            }
            if !self.proofs.bindings.contains_key(&id) {
                return Err(invalid());
            }
            if self.proofs.variable(id) || !self.eligible_local_type(ty, span, MAX_TYPES)? {
                continue;
            }
            if eligible.len() >= limit
                || !self
                    .flow
                    .spend(eligible.len().checked_ilog2().unwrap_or(0) as usize + 2)
            {
                return Err(budget());
            }
            eligible.insert(id);
        }
        Ok(eligible)
    }

    pub(super) fn eligible_local_type(
        &mut self,
        ty: &Type,
        span: Span,
        limit: usize,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof local-eligibility budget exhausted", span);
        let limit = limit.min(MAX_TYPES);
        if limit == 0 || !self.flow.spend(1) {
            return Err(budget());
        }
        let mut pending = vec![ty];
        while let Some(ty) = pending.pop() {
            if !self.flow.spend(1) {
                return Err(budget());
            }
            let mut push = |ty| {
                if pending.len() >= limit || !self.flow.spend(1) {
                    return Err(budget());
                }
                pending.push(ty);
                Ok(())
            };
            match ty {
                Type::Reference(_) | Type::Exclusive(_) | Type::Foundation(_) => return Ok(false),
                Type::Record { primary, fields } => {
                    push(primary.as_ref())?;
                    for field in fields {
                        if !self.flow.spend(1) {
                            return Err(budget());
                        }
                        if field.mutable {
                            return Ok(false);
                        }
                        if pending.len() >= limit || !self.flow.spend(1) {
                            return Err(budget());
                        }
                        pending.push(&field.ty);
                    }
                }
                Type::List { element, .. } => push(element.as_ref())?,
                Type::Union(members) => {
                    for ty in members {
                        push(ty)?;
                    }
                }
                Type::Null
                | Type::Never
                | Type::Bool
                | Type::Int { .. }
                | Type::Float { .. }
                | Type::String => {}
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
