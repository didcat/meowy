use super::super::effects::Effect;
use super::*;
use crate::check::dependencies::OperationKind;
use domain::Domain;
use versions::{Definition, Version};

#[derive(Debug)]
pub(super) struct Event<'a> {
    pub(super) source: &'a (usize, Effect),
    pub(super) ty: &'a hir::Type,
    pub(super) storage_ty: &'a hir::Type,
    pub(super) declared: Option<Domain>,
    pub(super) version: Option<Version>,
}

pub(super) type Events<'a> = BTreeMap<PointId, Event<'a>>;

impl Checker {
    pub(super) fn scalar_storage_events<'a>(
        &mut self,
        program: &'a hir::Program,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Events<'a>, usize)> {
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget(span));
        }
        self.program_signatures(program, reports, span)?;
        let mut events = Events::new();
        let mut parts = parts.min(MAX_EDGES);
        for (&point, source) in &reports.effects {
            let (owner, Effect::Storage { kind, .. }) = source else {
                continue;
            };
            let binding = match kind {
                OperationKind::Bind => self.binding_effect(reports, point, *owner, span)?,
                OperationKind::Write => self.write_effect(reports, point, *owner, span)?,
            }
            .ok_or_else(|| invalid(span))?;
            if !self.flow.spend(
                self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                    + 3,
            ) {
                return Err(budget(span));
            }
            if !reports.entries.contains_key(owner) {
                return Err(invalid(span));
            }
            if *kind == OperationKind::Bind && !self.proofs.mutable.contains(&binding.local) {
                continue;
            }
            let mut event = Event {
                source,
                ty: program
                    .locals
                    .get(binding.local)
                    .ok_or_else(|| invalid(span))?,
                storage_ty: program
                    .locals
                    .get(binding.storage)
                    .ok_or_else(|| invalid(span))?,
                declared: None,
                version: None,
            };
            event.declared = Domain::declared(event.ty, &mut self.flow, span)?;
            if !self.flow.spend(
                self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.fields.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.receivers.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                    + 7,
            ) {
                return Err(budget(span));
            }
            if event.declared.is_some()
                && binding.local == binding.storage
                && std::ptr::eq(event.ty, event.storage_ty)
                && !self.proofs.aliases.contains_key(&binding.local)
                && !self.proofs.fields.contains(&binding.local)
                && !self.proofs.receivers.contains(&binding.local)
                && !self.proofs.temporaries.contains_key(&binding.local)
            {
                event.version = Some(Version {
                    owner: event.source.0,
                    local: binding.storage,
                    definition: match kind {
                        OperationKind::Bind => Definition::Bind(point),
                        OperationKind::Write => Definition::Write(point),
                    },
                });
            }
            let weight = 1
                + event.declared.as_ref().map_or(0, Domain::weight)
                + usize::from(event.version.is_some());
            parts = parts.checked_sub(weight).ok_or_else(|| budget(span))?;
            if !self
                .flow
                .spend(events.len().checked_ilog2().unwrap_or(0) as usize + weight)
            {
                return Err(budget(span));
            }
            events.insert(point, event);
        }
        Ok((events, parts))
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-event budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-event identity mismatch", span)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
