use super::super::effects::Effect;
use super::*;
use crate::check::dependencies::{ScalarKind, grouped::MAX_GROUPS};
use std::collections::BTreeSet;
use versions::{Version, Versions};

mod reads;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Producer {
    Literal(PointId),
    Read(PointId, Version),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Input {
    pub(super) producer: Producer,
    pub(super) control: bool,
}

pub(super) type Inputs = BTreeMap<hir::LocalId, Input>;

impl Checker {
    pub(super) fn scalar_copy_inputs(
        &mut self,
        reports: &Reports,
        versions: &Versions,
        span: Span,
        parts: usize,
    ) -> Result<(Inputs, usize)> {
        if versions.len() > MAX_EDGES || !self.flow.spend(versions.len() + 1) {
            return Err(budget(span));
        }
        let mut inputs = Inputs::new();
        let mut parts = parts.min(MAX_EDGES);
        for (&local, slot) in versions {
            let Some(kind) = ScalarKind::of(slot.ty).filter(|_| slot.declared.is_some()) else {
                continue;
            };
            let Some((
                owner,
                Effect::Storage {
                    input: Some(point),
                    control,
                    ..
                },
            )) = slot.source
            else {
                continue;
            };
            if *owner != slot.version.owner || local != slot.version.local {
                return Err(invalid(span));
            }
            let Some(mut input) =
                self.scalar_copy_input(reports, versions, *owner, *point, kind, span)?
            else {
                continue;
            };
            input.control |= *control;
            let weight = 1 + usize::from(matches!(input.producer, Producer::Read(..)));
            parts = parts.checked_sub(weight).ok_or_else(|| budget(span))?;
            if !self
                .flow
                .spend(inputs.len().checked_ilog2().unwrap_or(0) as usize + weight)
            {
                return Err(budget(span));
            }
            inputs.insert(local, input);
        }
        Ok((inputs, parts))
    }

    pub(self) fn scalar_copy_input(
        &mut self,
        reports: &Reports,
        versions: &Versions,
        owner: usize,
        mut point: PointId,
        kind: ScalarKind,
        span: Span,
    ) -> Result<Option<Input>> {
        let mut seen = BTreeSet::new();
        let mut control = false;
        loop {
            if seen.len() >= MAX_GROUPS
                || !self.flow.spend(
                    seen.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + self.group_inputs.len().checked_ilog2().unwrap_or(0) as usize
                        + reports.effects.len().checked_ilog2().unwrap_or(0) as usize
                        + versions.len().checked_ilog2().unwrap_or(0) as usize
                        + 8,
                )
            {
                return Err(budget(span));
            }
            let source = self.points.get(point).ok_or_else(|| invalid(span))?;
            if !seen.insert(point)
                || !source.complete
                || source.owner != owner
                || !matches!(
                    source.kind,
                    PointKind::Expr | PointKind::And | PointKind::Or
                )
            {
                return Err(invalid(span));
            }
            let effect = reports.effects.get(&point);
            if let Some(group) = self.group_inputs.get(&point).copied() {
                if effect.is_some() {
                    return Err(invalid(span));
                }
                point = self.qualified_group_input(point, owner, group, span)?;
                continue;
            }
            let Some((reported, effect)) = effect else {
                return Ok(None);
            };
            if *reported != owner {
                return Err(invalid(span));
            }
            let next = match effect {
                Effect::Scalar(value) => {
                    self.validate_scalar_report(reports, point, owner, value, span)?;
                    return Ok((value.result && value.value.is_some() && value.ty == kind)
                        .then_some(Input {
                            producer: Producer::Literal(point),
                            control: control || value.control,
                        }));
                }
                Effect::Read {
                    control: marked, ..
                } => {
                    let Some(version) =
                        self.scalar_read_version(reports, versions, point, owner, span)?
                    else {
                        return Ok(None);
                    };
                    return Ok((ScalarKind::of(versions[&version.local].ty) == Some(kind))
                        .then_some(Input {
                            producer: Producer::Read(point, version),
                            control: control || *marked,
                        }));
                }
                Effect::Narrowing(value) => {
                    control |= value.control;
                    self.unchanged_narrowing_input(reports, point, owner, span)?
                }
                Effect::Typed(value) => {
                    control |= value.control;
                    self.unchanged_ascription_input(reports, point, owner, span)?
                }
                Effect::Coercion(value) => {
                    control |= value.control;
                    self.forward_coercion_input(reports, point, owner, span)?
                }
                _ => return Ok(None),
            };
            let Some(next) = next else { return Ok(None) };
            point = next;
        }
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-copy-source budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-copy-source identity mismatch", span)
}

#[cfg(test)]
mod tests;
