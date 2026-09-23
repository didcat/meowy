use super::{Checker, Diagnostic, Flow, MAX_FIELDS, Result, ShapeKey, Shapes, Span};
use std::collections::BTreeSet;

impl Shapes {
    pub(super) fn merged(&self, other: &Self, flow: &mut Flow, span: Span) -> Result<Self> {
        if !flow.spend(self.entries.len() + other.entries.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof record shape alias budget exhausted",
                span,
            ));
        }
        let keys = self
            .entries
            .keys()
            .chain(other.entries.keys())
            .collect::<BTreeSet<_>>();
        if keys.len() > MAX_FIELDS {
            return Err(Diagnostic::unsupported(
                "proof record shape alias capacity exhausted",
                span,
            ));
        }
        let mut result = Self::default();
        for key in keys {
            let variants = key
                .variants
                .iter()
                .map(|(at, ty)| (*at, ty))
                .collect::<Vec<_>>();
            let owned = ShapeKey::new(&key.fields, &variants, flow, span)?;
            let mut value = self.get(key).cloned().unwrap_or_default();
            value.merge(other.get(key).cloned().unwrap_or_default(), flow, span)?;
            result.insert(owned, value, flow, span)?;
        }
        Ok(result)
    }
}

impl Checker {
    pub(crate) fn merge_shape_alias(&mut self, id: usize, root: usize, span: Span) -> Result<()> {
        if id == root
            || (!self.record_shapes.contains_key(&id) && !self.record_shapes.contains_key(&root))
        {
            return Ok(());
        }
        let empty = Shapes::default();
        let prior = self.record_shapes.get(&root).unwrap_or(&empty);
        let next = self.record_shapes.get(&id).unwrap_or(&empty);
        let merged = prior.merged(next, &mut self.flow, span)?;
        self.record_shapes.insert(root, merged);
        self.record_shapes.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod emissions;
