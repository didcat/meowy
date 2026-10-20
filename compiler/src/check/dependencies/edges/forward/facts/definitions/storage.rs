use super::*;
use crate::check::dependencies::OperationKind;
use domain::Domain;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct Storage {
    pub(in super::super) version: Version,
    pub(in super::super) input: PointId,
    pub(in super::super) declared: Domain,
    pub(in super::super) control: bool,
}

impl Checker {
    pub(super) fn scalar_storage_definition(
        &mut self,
        definitions: &mut Definitions,
        location: Location,
        declared: (&hir::Type, Option<&Domain>),
        source: Option<&(usize, Effect)>,
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        let (point, expected) = match location.version.definition {
            Definition::Parameter(_) => return Ok(()),
            Definition::Bind(point) => (point, OperationKind::Bind),
            Definition::Write(point) => (point, OperationKind::Write),
        };
        if !self.flow.spend(6) {
            return Err(budget(span));
        }
        let Some((
            owner,
            Effect::Storage {
                kind,
                local,
                storage,
                input,
                control,
            },
        )) = source
        else {
            return Err(invalid(span));
        };
        if *owner != location.version.owner
            || *local != location.version.local
            || local != storage
            || *kind != expected
            || location.port != Port::Operation(point)
        {
            return Err(invalid(span));
        }
        let (Some(domain), Some(input)) = (declared.1, input) else {
            return Ok(());
        };
        if Domain::declared(declared.0, &mut self.flow, span)?.as_ref() != Some(domain) {
            return Err(invalid(span));
        }
        let weight = 1 + domain.weight();
        *parts = parts.checked_sub(weight).ok_or_else(|| budget(span))?;
        if !self.flow.spend(weight + 1) {
            return Err(budget(span));
        }
        let storage = Storage {
            version: location.version,
            input: *input,
            declared: domain.clone(),
            control: *control,
        };
        if definitions.storage.insert(point, storage).is_some() {
            return Err(invalid(span));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
