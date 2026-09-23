use super::{Checker, Diagnostic, Expr, MAX_DEPTH, Result, ShapeKey, Snapshot, Type};
use crate::check::dependencies::records::MAX_FIELDS;
use crate::hir::{Block, Stmt};
use std::collections::BTreeSet;

impl Checker {
    pub(super) fn block_shape_source(
        &mut self,
        value: &Expr,
        block: &Block,
        key: &ShapeKey,
        depth: usize,
    ) -> Result<Snapshot> {
        let Some(Type::Record { fields, .. }) = Self::origin_record(&value.ty) else {
            return Ok(Snapshot::default());
        };
        let Some((index, tail)) = key.fields.split_first() else {
            return Ok(Snapshot::default());
        };
        let Some(field) = fields.get(*index) else {
            return Ok(Snapshot::default());
        };
        if key.variants.iter().any(|(at, _)| *at == 0) {
            return Ok(Snapshot::default());
        }
        if !self.flow.spend(self.proofs.aliases.len()) {
            return Err(Diagnostic::unsupported(
                "proof record shape block budget exhausted",
                value.span,
            ));
        }
        let ids = self
            .proofs
            .aliases
            .iter()
            .filter(|(_, alias)| alias.target == block.id && alias.field == field.name)
            .map(|(id, _)| *id)
            .collect::<BTreeSet<_>>();
        if ids.is_empty() {
            return Ok(Snapshot::default());
        }
        if ids.len() > MAX_FIELDS || depth > MAX_DEPTH {
            return Err(Diagnostic::unsupported(
                "proof record shape block budget exhausted",
                value.span,
            ));
        }
        let variants = key
            .variants
            .iter()
            .map(|(at, ty)| (at - 1, ty))
            .collect::<Vec<_>>();
        let key = ShapeKey::new(tail, &variants, &mut self.flow, value.span)?;
        let mut pending = vec![(&block.stmts[..], 0)];
        let mut found = BTreeSet::new();
        let mut snapshot = Snapshot::empty();
        let mut visits = 0;
        while let Some((stmts, level)) = pending.pop() {
            if level > MAX_DEPTH {
                return Err(Diagnostic::unsupported(
                    "proof record shape block budget exhausted",
                    value.span,
                ));
            }
            for stmt in stmts {
                visits += 1;
                if visits > MAX_FIELDS || !self.flow.spend(1) {
                    return Err(Diagnostic::unsupported(
                        "proof record shape block budget exhausted",
                        value.span,
                    ));
                }
                match stmt {
                    Stmt::Statement { stmts, .. } => pending.push((stmts, level + 1)),
                    Stmt::If {
                        then, otherwise, ..
                    } => {
                        pending.push((then, level + 1));
                        pending.push((otherwise, level + 1));
                    }
                    Stmt::Bind { id, value: source } if ids.contains(id) => {
                        found.insert(*id);
                        let next = self.record_shape_source_at(source, &key, depth)?;
                        snapshot.merge(next, &mut self.flow, value.span)?;
                    }
                    _ => {}
                }
            }
        }
        let mut composed = false;
        if let Some(sources) = self.record_compositions.get(&block.id) {
            for id in sources {
                let Some(Type::Record { fields, .. }) = Self::origin_record(&self.locals[*id])
                else {
                    continue;
                };
                if !self.flow.spend(fields.len() + 1) {
                    return Err(Diagnostic::unsupported(
                        "proof record shape block budget exhausted",
                        value.span,
                    ));
                }
                composed |= fields.iter().any(|source| source.name == field.name);
            }
        }
        if found != ids || composed {
            snapshot.origins.complete = false;
            snapshot.cells.complete = false;
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests;
