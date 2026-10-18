use super::super::effects::Effect;
use super::*;
use domain::Domain;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Definition {
    Parameter(usize),
    Bind(PointId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Version {
    pub(super) owner: usize,
    pub(super) local: hir::LocalId,
    pub(super) definition: Definition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Origin {
    Parameter(hir::FunctionId, usize),
    Expression(usize, PointId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Value {
    pub(super) domain: Domain,
    pub(super) origin: Origin,
    pub(super) control: bool,
}

#[derive(Debug)]
pub(super) struct Slot<'a> {
    pub(super) version: Version,
    pub(super) ty: &'a hir::Type,
    pub(super) declared: Option<Domain>,
    pub(super) value: Option<Value>,
    pub(super) source: Option<&'a (usize, Effect)>,
}

pub(super) type Versions<'a> = BTreeMap<hir::LocalId, Slot<'a>>;

impl Checker {
    pub(super) fn scalar_version_seeds<'a>(
        &mut self,
        program: &'a hir::Program,
        reports: &'a Reports,
        declared: &declarations::Declarations,
        span: Span,
        parts: usize,
    ) -> Result<(Versions<'a>, usize)> {
        let mut versions = Versions::new();
        let mut parts = parts.min(MAX_EDGES);
        if declared.params.len() + reports.initializers.len() > MAX_EDGES
            || !self
                .flow
                .spend(declared.params.len() + reports.initializers.len() + 1)
        {
            return Err(budget(span));
        }
        for (&local, (function, position, seed)) in &declared.params {
            let domain = seed.domain.clone();
            let slot = Slot {
                version: Version {
                    owner: function.checked_add(1).ok_or_else(|| invalid(span))?,
                    local,
                    definition: Definition::Parameter(*position),
                },
                ty: program.locals.get(local).ok_or_else(|| invalid(span))?,
                declared: domain.clone(),
                value: domain.map(|domain| Value {
                    domain,
                    origin: Origin::Parameter(*function, *position),
                    control: false,
                }),
                source: None,
            };
            if !std::ptr::eq(slot.ty, seed.ty) {
                return Err(invalid(span));
            }
            self.insert_scalar_version(&mut versions, slot, &mut parts, span)?;
        }
        for (&local, init) in &reports.initializers {
            let binding = self
                .binding_effect(reports, init.statement, init.owner, span)?
                .ok_or_else(|| invalid(span))?;
            if !self.flow.spend(
                reports.eligible.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.effects.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.fields.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.receivers.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                    + 12,
            ) {
                return Err(budget(span));
            }
            if binding.local != local
                || binding.storage != local
                || binding.input != init.input
                || !reports.eligible.contains(&local)
                || !reports.entries.contains_key(&init.owner)
                || self.proofs.variable(local)
                || self.proofs.aliases.contains_key(&local)
                || self.proofs.receivers.contains(&local)
                || self.proofs.temporaries.contains_key(&local)
            {
                return Err(invalid(span));
            }
            let ty = program.locals.get(local).ok_or_else(|| invalid(span))?;
            let domain = Domain::declared(ty, &mut self.flow, span)?;
            self.insert_scalar_version(
                &mut versions,
                Slot {
                    version: Version {
                        owner: init.owner,
                        local,
                        definition: Definition::Bind(init.statement),
                    },
                    ty,
                    declared: domain,
                    value: None,
                    source: Some(
                        reports
                            .effects
                            .get(&init.statement)
                            .ok_or_else(|| invalid(span))?,
                    ),
                },
                &mut parts,
                span,
            )?;
        }
        Ok((versions, parts))
    }

    pub(self) fn insert_scalar_version<'a>(
        &mut self,
        versions: &mut Versions<'a>,
        slot: Slot<'a>,
        parts: &mut usize,
        span: Span,
    ) -> Result<()> {
        let weight = 1
            + usize::from(slot.source.is_some())
            + slot.declared.as_ref().map_or(0, Domain::weight)
            + slot.value.as_ref().map_or(0, |value| value.domain.weight());
        if !self
            .flow
            .spend(versions.len().checked_ilog2().unwrap_or(0) as usize * 2 + weight)
        {
            return Err(budget(span));
        }
        if versions.contains_key(&slot.version.local) {
            return Err(invalid(span));
        }
        *parts = parts.checked_sub(weight).ok_or_else(|| budget(span))?;
        versions.insert(slot.version.local, slot);
        Ok(())
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-version budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-version identity mismatch", span)
}

#[cfg(test)]
pub(super) mod tests;
