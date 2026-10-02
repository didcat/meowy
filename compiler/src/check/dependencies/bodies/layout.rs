use super::{Completion, MAX_BODY_FACTS, MAX_BODY_RESULTS, Walk, completion::Shape};
use crate::{ast::Span, check::Result, flow::Flow, hir};

pub(crate) const MAX_SLOTS: usize = MAX_BODY_RESULTS;
pub(crate) const MAX_NAMES: usize = 65_536;
pub(crate) const MAX_BODY_SLOTS: usize = MAX_BODY_FACTS;
pub(crate) const MAX_BODY_NAMES: usize = MAX_BODY_FACTS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Slot {
    pub(crate) field: Option<String>,
    pub(crate) mutable: bool,
    pub(crate) shape: Shape,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Layout {
    Stopped,
    Unknown,
    Slots(Vec<Slot>),
}

impl Layout {
    pub(crate) fn counts(&self) -> Option<(usize, usize)> {
        let Self::Slots(slots) = self else {
            return Some((0, 0));
        };
        if slots.is_empty() || slots.len() > MAX_SLOTS {
            return None;
        }
        let mut names = 0;
        for slot in slots {
            let size = slot.field.as_ref().map_or(0, String::len);
            if size > MAX_NAMES - names {
                return None;
            }
            names += size;
        }
        Some((slots.len(), names))
    }

    pub(crate) fn capture(
        ty: &hir::Type,
        flow: &mut Flow,
        used_slots: usize,
        used_names: usize,
        span: Span,
    ) -> Result<(Self, usize, usize)> {
        if !flow.spend(1) {
            return Err(Walk::budget(span));
        }
        if *ty == hir::Type::Never {
            return Ok((Self::Stopped, 0, 0));
        }
        if matches!(ty, hir::Type::Union(_))
            || (!matches!(ty, hir::Type::Record { .. })
                && Completion::of(ty).result == Shape::Other)
        {
            return Ok((Self::Unknown, 0, 0));
        }
        let (primary, fields) = match ty {
            hir::Type::Record { primary, fields } => (primary.as_ref(), fields.as_slice()),
            _ => (ty, &[][..]),
        };
        let count = fields
            .len()
            .checked_add(1)
            .ok_or_else(|| Walk::budget(span))?;
        if count > MAX_SLOTS || count > MAX_BODY_SLOTS.saturating_sub(used_slots) {
            return Err(Walk::budget(span));
        }
        if !flow.spend(count) {
            return Err(Walk::budget(span));
        }
        let mut names = 0;
        for field in fields {
            if field.name.len() > MAX_NAMES - names {
                return Err(Walk::budget(span));
            }
            names += field.name.len();
        }
        if names > MAX_BODY_NAMES.saturating_sub(used_names) || !flow.spend(count + names) {
            return Err(Walk::budget(span));
        }
        let mut slots = Vec::with_capacity(count);
        slots.push(Slot {
            field: None,
            mutable: false,
            shape: Completion::of(primary).result,
        });
        slots.extend(fields.iter().map(|field| Slot {
            field: Some(field.name.clone()),
            mutable: field.mutable,
            shape: Completion::of(&field.ty).result,
        }));
        Ok((Self::Slots(slots), count, names))
    }

    pub(crate) fn matches(&self, ty: &hir::Type) -> bool {
        match self {
            Self::Stopped => *ty == hir::Type::Never,
            Self::Unknown => {
                matches!(ty, hir::Type::Union(_))
                    || (!matches!(ty, hir::Type::Record { .. })
                        && Completion::of(ty).result == Shape::Other)
            }
            Self::Slots(slots) => {
                if matches!(
                    Completion::of(ty).result,
                    Shape::Never | Shape::Union { .. } | Shape::Other
                ) {
                    return false;
                }
                let (primary, fields) = match ty {
                    hir::Type::Record { primary, fields } => (primary.as_ref(), fields.as_slice()),
                    _ => (ty, &[][..]),
                };
                slots.len() == fields.len().saturating_add(1)
                    && slots.first().is_some_and(|slot| {
                        slot.field.is_none()
                            && !slot.mutable
                            && slot.shape == Completion::of(primary).result
                    })
                    && slots[1..].iter().zip(fields).all(|(slot, field)| {
                        slot.field.as_deref() == Some(field.name.as_str())
                            && slot.mutable == field.mutable
                            && slot.shape == Completion::of(&field.ty).result
                    })
            }
        }
    }
}

#[cfg(test)]
mod tests;
