use super::{Checker, Diagnostic, Expr, ExprKind, Origins, Result, Type};

impl Checker {
    pub(crate) fn record_source_origins(
        &mut self,
        value: &Expr,
        path: &[usize],
    ) -> Result<Origins> {
        let mut value = value;
        while let ExprKind::Coerce { value: inner } = &value.kind {
            let Some(record) = Self::origin_record(&value.ty) else {
                return Ok(Origins::default());
            };
            if inner.ty == Type::Null {
                return Ok(Origins {
                    roots: Default::default(),
                    complete: true,
                });
            }
            if Self::origin_record(&inner.ty) != Some(record) {
                return Ok(Origins::default());
            }
            value = inner;
        }
        if matches!(
            value.kind,
            ExprKind::Local(_) | ExprKind::Field { .. } | ExprKind::Deref(_)
        ) {
            return Ok(self.record_path_origins(value, path));
        }
        let ExprKind::Block(block) = &value.kind else {
            return Ok(Origins::default());
        };
        let Some(Type::Record { fields, .. }) = Self::origin_record(&value.ty) else {
            return Ok(Origins::default());
        };
        let Some((index, tail)) = path.split_first() else {
            return Ok(Origins::default());
        };
        if !self.flow.spend(self.proofs.aliases.len()) {
            return Err(Diagnostic::unsupported(
                "proof record origin budget exhausted",
                value.span,
            ));
        }
        let mut sources = Vec::new();
        for (id, alias) in &self.proofs.aliases {
            if alias.target != block.id || alias.field != fields[*index].name {
                continue;
            }
            let source = if tail.is_empty() {
                self.pointees.get(&alias.root)
            } else {
                self.record_pointees
                    .get(id)
                    .and_then(|fields| fields.get(tail))
            };
            sources.push(source);
        }
        if let Some(compositions) = self.record_compositions.get(&block.id) {
            if !self.flow.spend(compositions.len()) {
                return Err(Diagnostic::unsupported(
                    "proof record origin budget exhausted",
                    value.span,
                ));
            }
            for id in compositions {
                let Type::Record {
                    fields: source_fields,
                    ..
                } = &self.locals[*id]
                else {
                    unreachable!()
                };
                if !self.flow.spend(source_fields.len()) {
                    return Err(Diagnostic::unsupported(
                        "proof record origin budget exhausted",
                        value.span,
                    ));
                }
                if let Some(source_index) = source_fields
                    .iter()
                    .position(|field| field.name == fields[*index].name)
                {
                    let mut source_path = vec![source_index];
                    source_path.extend(tail);
                    sources.push(
                        self.record_pointees
                            .get(id)
                            .and_then(|fields| fields.get(&source_path)),
                    );
                }
            }
        }
        let mut origins = Origins::default();
        let mut found = false;
        for source in sources {
            if !self
                .flow
                .spend(source.map_or(0, |source| source.roots.len()) + 1)
            {
                return Err(Diagnostic::unsupported(
                    "proof record origin budget exhausted",
                    value.span,
                ));
            }
            let complete = source.is_some_and(|source| source.complete);
            origins.complete = if found {
                origins.complete && complete
            } else {
                complete
            };
            found = true;
            if let Some(source) = source {
                origins.roots.extend(&source.roots);
            }
            if origins.roots.len() > super::super::references::MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof record origin capacity exhausted",
                    value.span,
                ));
            }
        }
        Ok(origins)
    }
}

#[cfg(test)]
mod tests;
