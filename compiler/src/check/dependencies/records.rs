use super::{Checker, Origins};
use crate::check::Result;
use crate::diagnostic::Diagnostic;
use crate::hir::{Expr, ExprKind, Type};
use std::collections::BTreeMap;

pub(crate) const MAX_FIELDS: usize = 256;

impl Checker {
    pub(crate) fn field_origins(&self, value: &Expr, index: usize) -> Origins {
        let ExprKind::Local(id) = value.kind else {
            return Origins::default();
        };
        self.record_pointees
            .get(&id)
            .and_then(|fields| fields.get(&index))
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn track_record_references(&mut self, id: usize, value: &Expr) -> Result<()> {
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
            if field.mutable
                || !matches!(
                    field.ty.pointee(),
                    Some(Type::Bool | Type::Int { .. } | Type::Float { .. })
                )
            {
                continue;
            }
            let source = match &value.kind {
                ExprKind::Local(_) => self.field_origins(value, index),
                ExprKind::Block(block) => {
                    if !self.flow.spend(self.proofs.aliases.len()) {
                        return Err(Diagnostic::unsupported(
                            "proof record origin budget exhausted",
                            value.span,
                        ));
                    }
                    self.proofs
                        .aliases
                        .values()
                        .find(|alias| alias.target == block.id && alias.field == field.name)
                        .and_then(|alias| self.pointees.get(&alias.root))
                        .cloned()
                        .unwrap_or_default()
                }
                _ => Origins::default(),
            };
            if !self.flow.spend(source.roots.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "proof record origin budget exhausted",
                    value.span,
                ));
            }
            origins.insert(index, source);
        }
        if !origins.is_empty() {
            self.record_pointees.insert(id, origins);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
