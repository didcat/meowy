use super::{Checker, Diagnostic, Expr, ExprKind, Origins, Result, Type};

impl Checker {
    pub(crate) fn record_source_origins(
        &mut self,
        value: &Expr,
        path: &[usize],
    ) -> Result<Origins> {
        if matches!(value.kind, ExprKind::Local(_) | ExprKind::Field { .. }) {
            return Ok(self.record_path_origins(value, path));
        }
        let ExprKind::Block(block) = &value.kind else {
            return Ok(Origins::default());
        };
        let Type::Record { fields, .. } = &value.ty else {
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
        let mut origins = Origins::default();
        let mut found = false;
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
