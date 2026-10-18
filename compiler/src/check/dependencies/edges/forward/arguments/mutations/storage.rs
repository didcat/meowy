use super::*;
use crate::check::dependencies::ScalarKind;
use crate::hir::{ReferenceMode, Type};

impl Checker {
    pub(in super::super) fn mutation_place(
        &mut self,
        program: &Program,
        reports: &Reports,
        id: PointId,
        span: Span,
    ) -> Result<Option<ScalarKind>> {
        let budget = || Diagnostic::unsupported("proof call-storage budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-storage identity mismatch", span);
        if !self.flow.spend(
            self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.bindings.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                + 5,
        ) {
            return Err(budget());
        }
        let op = self.place_borrows.get(&id).ok_or_else(invalid)?;
        if op.mode != ReferenceMode::Exclusive || self.proofs.aliases.contains_key(&op.place.root) {
            return Ok(None);
        }
        let owner = op.owner;
        if !self.validate_place_borrow(reports, owner, Port::Normal(id), span)? {
            return Err(invalid());
        }
        let op = &self.place_borrows[&id];
        if program.locals.len() != reports.locals
            || !self.proofs.bindings.contains_key(&op.place.root)
            || op.storage != op.place.root
        {
            return Err(invalid());
        }
        let mut ty = program.locals.get(op.place.root).ok_or_else(invalid)?;
        if access::classify(ty, &mut self.flow, span)? != Access::Value {
            return Err(invalid());
        }
        let mut mutable = self.proofs.mutable.contains(&op.place.root);
        for (&index, &count) in op.place.fields.iter().zip(&op.counts) {
            let Type::Record { fields, .. } = ty else {
                return Err(invalid());
            };
            if fields.len() != count {
                return Err(invalid());
            }
            let field = fields.get(index).ok_or_else(invalid)?;
            ty = &field.ty;
            mutable = field.mutable;
        }
        let Some(kind @ (ScalarKind::Bool | ScalarKind::Int { .. } | ScalarKind::Float { .. })) =
            ScalarKind::of(ty)
        else {
            return Err(invalid());
        };
        if !mutable || op.scalar != Some(kind) {
            return Err(invalid());
        }
        Ok(Some(kind))
    }
}

#[cfg(test)]
mod tests;
