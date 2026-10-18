use super::{effects::Effect, entries::Reports, *};
use crate::{check::dependencies::OperationKind, hir};

mod storage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Binding {
    pub(super) local: hir::LocalId,
    pub(super) storage: hir::LocalId,
    pub(super) input: Option<PointId>,
}

impl Checker {
    #[cfg(test)]
    pub(super) fn validate_bindings(&mut self, reports: &Reports, span: Span) -> Result<()> {
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof binding budget exhausted",
                span,
            ));
        }
        for (&id, (owner, effect)) in &reports.effects {
            if matches!(effect, Effect::Storage { .. }) {
                self.binding_effect(reports, id, *owner, span)?;
            }
        }
        Ok(())
    }

    pub(super) fn binding_effect(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<Binding>> {
        self.storage_effect(reports, id, owner, OperationKind::Bind, span)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
