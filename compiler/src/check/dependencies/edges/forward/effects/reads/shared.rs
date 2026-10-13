use super::*;
use crate::check::dependencies::{
    bodies::{Completion, completion::Shape},
    grouped::MAX_GROUPS,
    operations::SharedRecord,
};

impl Checker {
    pub(in super::super::super) fn read_shared_initializer_input(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        record: SharedRecord,
        span: Span,
    ) -> Result<Option<PointId>> {
        let budget = || Diagnostic::unsupported("proof shared-read budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof shared-read identity mismatch", span);
        let work = [
            reports.effects.len(),
            reports.shared_initializers.len(),
            reports.entries.len(),
            self.proofs.mutable.len(),
            self.proofs.fields.len(),
            self.proofs.aliases.len(),
            self.proofs.receivers.len(),
            self.proofs.temporaries.len(),
            self.operations.len(),
        ]
        .into_iter()
        .fold(15, |work, len| {
            work + len.checked_ilog2().unwrap_or(0) as usize
        });
        if !self.flow.spend(work) {
            return Err(budget());
        }
        let Some((
            reported,
            effect @ Effect::Read {
                local,
                storage,
                normal,
                ..
            },
        )) = reports.effects.get(&id)
        else {
            return Ok(None);
        };
        if *reported != owner || self.read_effect(reports, id, owner, span)? != *effect {
            return Err(invalid());
        }
        if !normal
            || local != storage
            || self.proofs.variable(*local)
            || self.proofs.aliases.contains_key(local)
            || self.proofs.receivers.contains(local)
            || self.proofs.temporaries.contains_key(local)
        {
            return Ok(None);
        }
        let Some(row) = reports.shared_initializers.get(local) else {
            return Ok(None);
        };
        if row.record != record
            || row.init.owner != owner
            || !reports.entries.contains_key(&owner)
            || !(Completion {
                normal: true,
                result: Shape::SharedScalar(record.primary),
            })
            .valid()
            || record.fields >= crate::check::dependencies::bodies::layout::MAX_SLOTS
        {
            return Err(invalid());
        }
        let Some(binding) = self.binding_effect(reports, row.init.statement, owner, span)? else {
            return Ok(None);
        };
        if binding.local != *local
            || binding.storage != *storage
            || binding.input != row.init.input
            || self.operations[&row.init.statement].shared != Some(record)
        {
            return Err(invalid());
        }
        let block = self.points[row.init.statement].block.ok_or_else(invalid)?;
        self.shared_local_scope(id, block, owner, span)?;
        Ok(row.init.input)
    }

    pub(self) fn shared_local_scope(
        &mut self,
        mut id: PointId,
        target: crate::hir::BlockId,
        owner: usize,
        span: Span,
    ) -> Result<()> {
        let invalid =
            || Diagnostic::unsupported("proof shared-local scope identity mismatch", span);
        let mut seen = std::collections::BTreeSet::new();
        loop {
            if seen.len() >= MAX_GROUPS
                || !self.flow.spend(
                    self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                        + seen.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + 10,
                )
            {
                return Err(Diagnostic::unsupported(
                    "proof shared-local scope budget exhausted",
                    span,
                ));
            }
            if !seen.insert(id) {
                return Err(invalid());
            }
            let point = self.points.get(id).ok_or_else(invalid)?;
            let block = point.block.ok_or_else(invalid)?;
            let body = self.bodies.get(&block).ok_or_else(invalid)?;
            if !point.complete
                || point.owner != owner
                || body.owner != owner
                || point.span.start > point.span.end
            {
                return Err(invalid());
            }
            if block == target {
                return Ok(());
            }
            id = body.parent.ok_or_else(invalid)?;
        }
    }
}
