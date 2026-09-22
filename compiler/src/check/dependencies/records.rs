use super::{Checker, Origins};
use crate::check::Result;
use crate::diagnostic::Diagnostic;
use crate::hir::{Expr, ExprKind, Type};
use std::collections::BTreeMap;

pub(crate) const MAX_FIELDS: usize = 256;

impl Checker {
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
        index: usize,
        value: &Expr,
    ) -> Result<()> {
        if !matches!(
            value.ty.pointee(),
            Some(Type::Bool | Type::Int { .. } | Type::Float { .. })
        ) {
            return Ok(());
        }
        let mut origins = self.reference_origins(value);
        let prior = self
            .record_pointees
            .get(&id)
            .and_then(|fields| fields.get([index].as_slice()));
        if index >= MAX_FIELDS
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
            .insert(vec![index], origins);
        Ok(())
    }

    pub(crate) fn track_record_references(
        &mut self,
        id: usize,
        value: &Expr,
        merge: bool,
    ) -> Result<()> {
        let Type::Record { fields, .. } = &value.ty else {
            return Ok(());
        };
        if !fields.iter().any(|field| field.ty.pointee().is_some()) {
            return Ok(());
        }
        if fields.len() > MAX_FIELDS || !self.flow.spend(fields.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof record origin budget exhausted",
                value.span,
            ));
        }
        let mut origins = BTreeMap::new();
        for (index, field) in fields.iter().enumerate() {
            if !matches!(
                field.ty.pointee(),
                Some(Type::Bool | Type::Int { .. } | Type::Float { .. })
            ) {
                continue;
            }
            let mut source = self.record_source_origins(value, &[index])?;
            let prior = merge
                .then(|| {
                    self.record_pointees
                        .get(&id)
                        .and_then(|fields| fields.get([index].as_slice()))
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
            origins.insert(vec![index], source);
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
