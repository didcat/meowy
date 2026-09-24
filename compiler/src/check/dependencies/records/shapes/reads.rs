use super::{Checker, Diagnostic, Expr, MAX_DEPTH, Result, ShapeKey, Snapshot, Type};
use crate::hir::ExprKind;

impl Checker {
    pub(crate) fn record_shape_snapshot(
        &mut self,
        value: &Expr,
        path: &[usize],
    ) -> Result<Option<Snapshot>> {
        if path.len() > MAX_DEPTH {
            return Err(Diagnostic::unsupported(
                "proof record shape read budget exhausted",
                value.span,
            ));
        }
        let mut base = value;
        let mut fields = path.iter().rev().copied().collect::<Vec<_>>();
        let mut variants = Vec::new();
        let mut depth = path.len();
        let root = loop {
            if depth > MAX_DEPTH || !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "proof record shape read budget exhausted",
                    value.span,
                ));
            }
            match &base.kind {
                ExprKind::Field {
                    value: inner,
                    index,
                } => {
                    fields.push(*index);
                    base = inner;
                }
                ExprKind::Coerce { value: inner } => {
                    let Some(record) = Self::origin_record(&base.ty) else {
                        return Ok(None);
                    };
                    if Self::origin_record(&inner.ty) != Some(record) {
                        let Type::Union(members) = &inner.ty else {
                            return Ok(None);
                        };
                        if !self.flow.spend(members.len()) {
                            return Err(Diagnostic::unsupported(
                                "proof record shape read budget exhausted",
                                value.span,
                            ));
                        }
                        let Some(member) = members
                            .iter()
                            .find(|member| Self::origin_record(member) == Some(record))
                        else {
                            return Ok(None);
                        };
                        variants.push((fields.len(), member));
                    }
                    base = inner;
                }
                ExprKind::Local(id) => break *id,
                ExprKind::Deref(inner) => {
                    let Some(place) = self.shape_temporary(inner)? else {
                        return Ok(None);
                    };
                    fields.extend(place.fields.iter().rev());
                    break place.root;
                }
                _ => return Ok(None),
            }
            depth += 1;
        };
        if variants.is_empty() {
            return Ok(None);
        }
        fields.reverse();
        let variants = variants
            .into_iter()
            .rev()
            .map(|(at, ty)| (fields.len() - at, ty))
            .collect::<Vec<_>>();
        let key = ShapeKey::new(&fields, &variants, &mut self.flow, value.span)?;
        let root = self.origin_id(root);
        let snapshot = self
            .record_shapes
            .get(&root)
            .and_then(|shapes| shapes.get(&key));
        let work = snapshot.map_or(1, |value| {
            value.origins.roots.len()
                + value
                    .cells
                    .places
                    .iter()
                    .map(|(_, path)| path.len() + 1)
                    .sum::<usize>()
                + 1
        });
        if !self.flow.spend(work) {
            return Err(Diagnostic::unsupported(
                "proof record shape read budget exhausted",
                value.span,
            ));
        }
        Ok(Some(snapshot.cloned().unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests;
