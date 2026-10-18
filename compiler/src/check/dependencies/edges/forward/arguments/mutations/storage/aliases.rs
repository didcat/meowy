use super::*;
use crate::borrow::Backing;
use crate::check::dependencies::edges::forward::arguments::program::blocks::Blocks;

impl Checker {
    pub(in super::super::super) fn mutation_slot(
        &mut self,
        program: &Program,
        reports: &Reports,
        blocks: &Blocks<'_>,
        id: PointId,
        span: Span,
    ) -> Result<Option<ScalarKind>> {
        let budget = || Diagnostic::unsupported("proof call-alias budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-alias identity mismatch", span);
        if !self.flow.spend(
            self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                + blocks.len().checked_ilog2().unwrap_or(0) as usize
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + 8,
        ) {
            return Err(budget());
        }
        let op = self.place_borrows.get(&id).ok_or_else(invalid)?;
        if op.mode != ReferenceMode::Exclusive {
            return Ok(None);
        }
        let Some(alias) = self.proofs.aliases.get(&op.place.root) else {
            return Ok(None);
        };
        if alias.backing == Some(Backing::Discarded) {
            return Ok(None);
        }
        let root = self.proofs.aliases.get(&alias.root).ok_or_else(invalid)?;
        let (owner, block) = blocks.get(&alias.target).ok_or_else(invalid)?;
        let local = program.locals.get(op.place.root).ok_or_else(invalid)?;
        let storage = program.locals.get(alias.root).ok_or_else(invalid)?;
        let weight = crate::borrow_contract::type_weight(&block.ty, &mut self.flow, span)?;
        let local_weight = crate::borrow_contract::type_weight(local, &mut self.flow, span)?;
        let root_weight = crate::borrow_contract::type_weight(storage, &mut self.flow, span)?;
        if !self.flow.spend(
            weight
                .saturating_add(local_weight)
                .saturating_add(root_weight)
                .saturating_add(alias.field.len())
                .saturating_add(root.field.len()),
        ) {
            return Err(budget());
        }
        if alias.backing != Some(Backing::Result)
            || root.backing != Some(Backing::Result)
            || alias.exclusive.is_none()
            || root.root != alias.root
            || root.target != alias.target
            || root.field != alias.field
            || root.mutable != alias.mutable
            || self.proofs.mutable.contains(&op.place.root) != alias.mutable
            || *owner != op.owner
            || block.id != alias.target
            || !self
                .bodies
                .get(&alias.target)
                .is_some_and(|body| body.owner == *owner && body.layout.matches(&block.ty))
            || storage != local
        {
            return Err(invalid());
        }
        let Type::Record { fields, .. } = &block.ty else {
            return Err(invalid());
        };
        if !self.flow.spend(
            fields
                .len()
                .saturating_mul(alias.field.len().saturating_add(1)),
        ) {
            return Err(budget());
        }
        let field = fields
            .iter()
            .find(|field| field.name == alias.field)
            .ok_or_else(invalid)?;
        if field.mutable != alias.mutable || &field.ty != local {
            return Err(invalid());
        }
        let owner = *owner;
        let emissions = [alias.emission, root.emission];
        for emission in emissions {
            if self
                .emission_effect_stage(reports, owner, Port::Emission(emission), span)?
                .is_none()
            {
                return Err(invalid());
            }
        }
        self.mutation_scalar(program, reports, id, span)
    }
}

#[cfg(test)]
mod tests;
