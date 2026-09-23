use super::{Checker, Origins};
use crate::check::Result;
use crate::diagnostic::Diagnostic;
use crate::hir::{Expr, ExprKind, Type};
use std::collections::BTreeMap;

pub(crate) const MAX_FIELDS: usize = 256;
pub(crate) const MAX_DEPTH: usize = 32;

impl Checker {
    pub(crate) fn origin_record(ty: &Type) -> Option<&Type> {
        if matches!(ty, Type::Record { .. }) {
            return Some(ty);
        }
        let Type::Union(members) = ty else {
            return None;
        };
        let mut records = members.iter().filter(|ty| **ty != Type::Null);
        let record = records.next()?;
        (records.next().is_none() && matches!(record, Type::Record { .. })).then_some(record)
    }

    pub(crate) fn field_origins(&self, value: &Expr, index: usize) -> Origins {
        self.record_path_origins(value, &[index])
    }

    pub(crate) fn record_path_origins(&self, value: &Expr, path: &[usize]) -> Origins {
        let mut value = value;
        let mut path = path.iter().rev().copied().collect::<Vec<_>>();
        loop {
            match &value.kind {
                ExprKind::Field { value: base, index } => {
                    path.push(*index);
                    value = base;
                }
                ExprKind::Coerce { value: inner }
                    if Self::origin_record(&value.ty).is_some()
                        && Self::origin_record(&value.ty) == Self::origin_record(&inner.ty) =>
                {
                    value = inner;
                }
                ExprKind::Deref(inner) => {
                    let Some(id) = Self::temporary_storage(inner) else {
                        return Origins::default();
                    };
                    path.reverse();
                    return self
                        .record_pointees
                        .get(&id)
                        .and_then(|fields| fields.get(&path))
                        .cloned()
                        .unwrap_or_default();
                }
                ExprKind::Local(id) => {
                    path.reverse();
                    return self
                        .record_pointees
                        .get(id)
                        .and_then(|fields| fields.get(&path))
                        .cloned()
                        .unwrap_or_default();
                }
                _ => return Origins::default(),
            }
        }
    }

    pub(crate) fn write_reference_field(
        &mut self,
        id: usize,
        path: &[usize],
        value: &Expr,
    ) -> Result<()> {
        if !Self::origin_reference(&value.ty) {
            return Ok(());
        }
        let mut origins = self.reference_origins(value);
        let prior = self
            .record_pointees
            .get(&id)
            .and_then(|fields| fields.get(path));
        if path.len() > MAX_DEPTH
            || path.iter().any(|index| *index >= MAX_FIELDS)
            || !self
                .flow
                .spend(origins.roots.len() + prior.map_or(0, |prior| prior.roots.len()) + 1)
        {
            return Err(Diagnostic::unsupported(
                "proof record origin budget exhausted",
                value.span,
            ));
        }
        origins.complete &= prior.is_some_and(|prior| prior.complete);
        if let Some(prior) = prior {
            origins.roots.extend(&prior.roots);
        }
        if origins.roots.len() > super::references::MAX_ROOTS {
            return Err(Diagnostic::unsupported(
                "proof record origin capacity exhausted",
                value.span,
            ));
        }
        self.record_pointees
            .entry(id)
            .or_default()
            .insert(path.to_vec(), origins);
        Ok(())
    }

    pub(crate) fn record_paths(&mut self, value: &Expr) -> Result<Vec<Vec<usize>>> {
        let mut pending = vec![(Vec::new(), &value.ty)];
        let mut paths = Vec::new();
        let mut count = 0;
        while let Some((path, ty)) = pending.pop() {
            if !ty.has_reference() {
                continue;
            }
            if let Some(Type::Record { fields, .. }) = Self::origin_record(ty) {
                count += fields.len();
                if count > MAX_FIELDS
                    || path.len() >= MAX_DEPTH
                    || !self.flow.spend(fields.len() + 1)
                {
                    return Err(Diagnostic::unsupported(
                        "proof record origin budget exhausted",
                        value.span,
                    ));
                }
                for (index, field) in fields.iter().enumerate() {
                    let mut child = path.clone();
                    child.push(index);
                    pending.push((child, &field.ty));
                }
            } else if Self::origin_reference(ty) {
                paths.push(path);
            }
        }
        Ok(paths)
    }

    pub(crate) fn track_record_references(
        &mut self,
        id: usize,
        value: &Expr,
        merge: bool,
    ) -> Result<()> {
        self.track_record_prefix(id, &[], value, merge)
    }

    pub(crate) fn track_record_prefix(
        &mut self,
        id: usize,
        prefix: &[usize],
        value: &Expr,
        merge: bool,
    ) -> Result<()> {
        if Self::origin_record(&value.ty).is_none() || !value.ty.has_reference() {
            return Ok(());
        }
        let paths = self.record_paths(value)?;
        let mut origins = BTreeMap::new();
        if !prefix.is_empty()
            && let Some(prior) = self.record_pointees.get(&id)
        {
            if !self.flow.spend(
                prior
                    .iter()
                    .map(|(path, origins)| path.len() + origins.roots.len() + 1)
                    .sum(),
            ) {
                return Err(Diagnostic::unsupported(
                    "proof record origin budget exhausted",
                    value.span,
                ));
            }
            origins = prior.clone();
        }
        for path in paths {
            let mut key = prefix.to_vec();
            key.extend(&path);
            if key.len() > MAX_DEPTH {
                return Err(Diagnostic::unsupported(
                    "proof record origin budget exhausted",
                    value.span,
                ));
            }
            let mut source = self.record_source_origins(value, &path)?;
            let prior = merge
                .then(|| {
                    self.record_pointees
                        .get(&id)
                        .and_then(|fields| fields.get(&key))
                })
                .flatten();
            if !self
                .flow
                .spend(source.roots.len() + prior.map_or(0, |prior| prior.roots.len()) + 1)
            {
                return Err(Diagnostic::unsupported(
                    "proof record origin budget exhausted",
                    value.span,
                ));
            }
            if let Some(prior) = prior {
                source.complete &= prior.complete;
                source.roots.extend(&prior.roots);
            } else if merge {
                source.complete = false;
            }
            if source.roots.len() > super::references::MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof record origin capacity exhausted",
                    value.span,
                ));
            }
            origins.insert(key, source);
        }
        if origins.len() > MAX_FIELDS {
            return Err(Diagnostic::unsupported(
                "proof record origin capacity exhausted",
                value.span,
            ));
        }
        if !origins.is_empty() {
            self.record_pointees.insert(id, origins);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod writes;

mod sources;

#[cfg(test)]
mod nested;

#[cfg(test)]
mod coercions;

#[cfg(test)]
mod views;
