use super::{Checker, Diagnostic, Expr, Flow, Result, ShapeKey, Shapes, Span};

impl Shapes {
    pub(super) fn merged_prefix(
        &self,
        next: &Self,
        prefix: &[usize],
        flow: &mut Flow,
        span: Span,
    ) -> Result<Self> {
        ShapeKey::new(prefix, &[], flow, span)?;
        let mut incoming = Self::default();
        for (key, value) in &next.entries {
            let mut fields = prefix.to_vec();
            fields.extend(&key.fields);
            let variants = key
                .variants
                .iter()
                .map(|(at, ty)| (at + prefix.len(), ty))
                .collect::<Vec<_>>();
            let key = ShapeKey::new(&fields, &variants, flow, span)?;
            incoming.insert(key, value.clone(), flow, span)?;
        }
        let mut affected = Self::default();
        let mut result = Self::default();
        for (key, value) in &self.entries {
            let selected = key.fields.starts_with(prefix);
            if selected && key.variants.iter().any(|(at, _)| *at < prefix.len()) {
                return Err(Diagnostic::unsupported(
                    "proof record shape write crosses a union selection",
                    span,
                ));
            }
            let variants = key
                .variants
                .iter()
                .map(|(at, ty)| (*at, ty))
                .collect::<Vec<_>>();
            let key = ShapeKey::new(&key.fields, &variants, flow, span)?;
            let target = if selected { &mut affected } else { &mut result };
            target.insert(key, value.clone(), flow, span)?;
        }
        let merged = affected.merged(&incoming, flow, span)?;
        for (key, value) in merged.entries {
            result.insert(key, value, flow, span)?;
        }
        Ok(result)
    }
}

impl Checker {
    pub(crate) fn track_shape_prefix(
        &mut self,
        id: usize,
        prefix: &[usize],
        value: &Expr,
    ) -> Result<()> {
        let next = self.record_shape_values(value, true)?;
        let root = self.origin_id(id);
        let empty = Shapes::default();
        let prior = self.record_shapes.get(&root).unwrap_or(&empty);
        if !self.flow.spend(prior.entries.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof record shape prefix budget exhausted",
                value.span,
            ));
        }
        if next.entries.is_empty()
            && !prior
                .entries
                .keys()
                .any(|key| key.fields.starts_with(prefix))
        {
            return Ok(());
        }
        let shapes = prior.merged_prefix(&next, prefix, &mut self.flow, value.span)?;
        if !shapes.entries.is_empty() {
            self.record_shapes.insert(root, shapes);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
