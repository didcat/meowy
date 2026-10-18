use super::*;
use crate::borrow_value::{Origin, Projection, Source};
use crate::check::dependencies::edges::forward::arguments::program::blocks::Blocks;
use std::collections::btree_map::Entry;
use std::ops::Bound::{Excluded, Unbounded};

pub(in super::super) type Storage = BTreeMap<(usize, Source), (PointId, ScalarKind)>;

impl Checker {
    pub(in super::super) fn mutation_storage(
        &mut self,
        program: &Program,
        reports: &Reports,
        blocks: &Blocks<'_>,
        span: Span,
        parts: usize,
    ) -> Result<(Storage, usize)> {
        let budget = || Diagnostic::unsupported("proof call-storage budget exhausted", span);
        if self.place_borrows.len() > MAX_EDGES || !self.flow.spend(1) {
            return Err(budget());
        }
        let mut parts = parts.min(MAX_EDGES);
        let mut storage = Storage::new();
        let mut next = self.place_borrows.first_key_value().map(|(&id, _)| id);
        while let Some(id) = next {
            if !self
                .flow
                .spend(self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget());
            }
            next = self
                .place_borrows
                .range((Excluded(id), Unbounded))
                .next()
                .map(|(&id, _)| id);
            let Some(kind) = self.mutation_storage_kind(program, reports, blocks, id, span)? else {
                continue;
            };
            let (key, cost) = self.mutation_storage_key(id, span)?;
            if !self.flow.spend(
                cost.saturating_mul(storage.len().checked_ilog2().unwrap_or(0) as usize * 2 + 1),
            ) {
                return Err(budget());
            }
            match storage.entry(key) {
                Entry::Vacant(row) => {
                    parts = parts.checked_sub(cost).ok_or_else(budget)?;
                    row.insert((id, kind));
                }
                Entry::Occupied(row) if row.get().1 != kind => {
                    return Err(Diagnostic::unsupported(
                        "proof call-storage identity mismatch",
                        span,
                    ));
                }
                Entry::Occupied(_) => (),
            }
        }
        Ok((storage, parts))
    }

    pub(super) fn mutation_storage_kind(
        &mut self,
        program: &Program,
        reports: &Reports,
        blocks: &Blocks<'_>,
        id: PointId,
        span: Span,
    ) -> Result<Option<ScalarKind>> {
        match self.mutation_place(program, reports, id, span)? {
            Some(kind) => Ok(Some(kind)),
            None => self.mutation_slot(program, reports, blocks, id, span),
        }
    }

    pub(super) fn mutation_storage_key(
        &mut self,
        id: PointId,
        span: Span,
    ) -> Result<((usize, Source), usize)> {
        let invalid = || Diagnostic::unsupported("proof call-storage identity mismatch", span);
        let op = self.place_borrows.get(&id).ok_or_else(invalid)?;
        if op.place.fields.len() > crate::list::MAX_WRITE_PATH
            || !self.flow.spend(
                op.place.fields.len()
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + 2,
            )
        {
            return Err(Diagnostic::unsupported(
                "proof call-storage budget exhausted",
                span,
            ));
        }
        let fields = op
            .place
            .fields
            .iter()
            .copied()
            .map(Projection::Field)
            .collect();
        let (source, cost) = match self.proofs.aliases.get(&op.place.root) {
            Some(alias) => (
                Source::Slot {
                    target: alias.target,
                    root: alias.root,
                    view: op.place.root,
                    fields,
                },
                4,
            ),
            None => (
                Source::Local {
                    id: op.place.root,
                    fields,
                },
                1,
            ),
        };
        Ok(((op.owner, source), op.place.fields.len() + cost))
    }

    pub(in super::super) fn mutation_origin(
        &mut self,
        ctx: (&Program, &Reports, &Blocks<'_>, &Storage),
        owner: usize,
        kind: ScalarKind,
        origin: &Origin,
        span: Span,
    ) -> Result<Option<PointId>> {
        let (program, reports, blocks, storage) = ctx;
        let invalid = || Diagnostic::unsupported("proof call-storage identity mismatch", span);
        if !self.flow.spend(
            origin
                .weight()
                .saturating_mul(storage.len().checked_ilog2().unwrap_or(0) as usize + 2),
        ) {
            return Err(Diagnostic::unsupported(
                "proof call-storage budget exhausted",
                span,
            ));
        }
        if !origin.component.is_empty() {
            return Ok(None);
        }
        let fields = match &origin.source {
            Source::Local { fields, .. } | Source::Slot { fields, .. } => fields,
            _ => return Ok(None),
        };
        if fields
            .iter()
            .any(|field| matches!(field, Projection::Element))
        {
            return Ok(None);
        }
        let key = (owner, origin.source.clone());
        let Some(&(id, stored)) = storage.get(&key) else {
            return Ok(None);
        };
        if stored != kind
            || self.mutation_storage_kind(program, reports, blocks, id, span)? != Some(kind)
            || self.mutation_storage_key(id, span)?.0 != key
        {
            return Err(invalid());
        }
        Ok(Some(id))
    }
}

#[cfg(test)]
mod tests;
