use super::*;
use domain::Domain;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct Record {
    pub(in super::super) storage: Storage,
    pub(in super::super) block: hir::BlockId,
}

impl Checker {
    pub(super) fn scalar_record_definition(
        &mut self,
        reports: &Reports,
        definitions: &mut Definitions,
        location: Location,
        ty: &hir::Type,
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        let Definition::Bind(point) = location.version.definition else {
            return Ok(());
        };
        let local = location.version.local;
        if !self
            .flow
            .spend(reports.initializers.len().checked_ilog2().unwrap_or(0) as usize + 5)
        {
            return Err(budget(span));
        }
        let init = reports
            .initializers
            .get(&local)
            .ok_or_else(|| invalid(span))?;
        let binding = self
            .binding_effect(reports, point, location.version.owner, span)?
            .ok_or_else(|| invalid(span))?;
        if init.statement != point
            || init.owner != location.version.owner
            || binding.local != local
            || binding.storage != local
            || init.input != binding.input
            || location.port != Port::Operation(point)
            || definitions.storage.contains_key(&point)
        {
            return Err(invalid(span));
        }
        let Some(input) = init.input else {
            return Ok(());
        };
        let Some(slot) = self.primary_slot(reports, input, init.owner, span)? else {
            return Ok(());
        };
        let Some(scalar) = self.scalar_scope_type(slot.block, span)? else {
            return Ok(());
        };
        let body = &self.bodies[&slot.block];
        let counts = body.layout.counts().ok_or_else(|| invalid(span))?;
        if !self.flow.spend(counts.0 + counts.1 + 3) {
            return Err(budget(span));
        }
        if !matches!(ty, hir::Type::Record { .. })
            || !body.layout.matches(ty)
            || body.owner != init.owner
            || slot.index != 0
            || scalar.nullable
        {
            return Err(invalid(span));
        }
        let declared =
            Domain::scalar(scalar.kind, &mut self.flow, span)?.ok_or_else(|| invalid(span))?;
        let Effect::Storage { control, .. } = reports.effects[&point].1 else {
            return Err(invalid(span));
        };
        let weight = 2 + declared.weight();
        if !self.flow.spend(weight + 1) {
            return Err(budget(span));
        }
        *parts = parts.checked_sub(weight).ok_or_else(|| budget(span))?;
        if definitions
            .records
            .insert(
                point,
                Record {
                    block: slot.block,
                    storage: Storage {
                        version: location.version,
                        input,
                        declared,
                        control,
                    },
                },
            )
            .is_some()
        {
            return Err(invalid(span));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
