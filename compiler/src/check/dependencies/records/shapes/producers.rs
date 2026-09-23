use super::{Checker, Diagnostic, Expr, MAX_DEPTH, Result, ShapeKey, Snapshot, Type};
use crate::check::dependencies::references::MAX_ROOTS;
use crate::hir::ExprKind;
use crate::{ast::Span, flow::Flow};

impl Snapshot {
    pub(super) fn merge(&mut self, next: Self, flow: &mut Flow, span: Span) -> Result<()> {
        let work = next.origins.roots.len()
            + next
                .cells
                .places
                .iter()
                .map(|(_, path)| path.len() + 1)
                .sum::<usize>()
            + 1;
        if !flow.spend(work) {
            return Err(Diagnostic::unsupported(
                "proof record shape block budget exhausted",
                span,
            ));
        }
        self.origins.complete &= next.origins.complete;
        self.origins.roots.extend(next.origins.roots);
        self.cells.complete &= next.cells.complete;
        self.cells.places.extend(next.cells.places);
        if self.origins.roots.len() > MAX_ROOTS || self.cells.places.len() > MAX_ROOTS {
            return Err(Diagnostic::unsupported(
                "proof record shape block capacity exhausted",
                span,
            ));
        }
        Ok(())
    }

    pub(super) fn empty() -> Self {
        let mut value = Self::default();
        value.origins.complete = true;
        value.cells.complete = true;
        value
    }
}

impl Checker {
    pub(super) fn record_shape_source(&mut self, value: &Expr, key: &ShapeKey) -> Result<Snapshot> {
        self.record_shape_source_at(value, key, 0)
    }

    pub(super) fn record_shape_source_at(
        &mut self,
        value: &Expr,
        key: &ShapeKey,
        depth: usize,
    ) -> Result<Snapshot> {
        let mut base = value;
        let mut key = ShapeKey::new(
            &key.fields,
            &key.variants
                .iter()
                .map(|(at, ty)| (*at, ty))
                .collect::<Vec<_>>(),
            &mut self.flow,
            value.span,
        )?;
        for depth in depth..MAX_DEPTH {
            if !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "proof record shape source budget exhausted",
                    value.span,
                ));
            }
            match &base.kind {
                ExprKind::Local(id) => {
                    let snapshot = self
                        .record_shapes
                        .get(id)
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
                            "proof record shape source budget exhausted",
                            value.span,
                        ));
                    }
                    return Ok(snapshot.cloned().unwrap_or_default());
                }
                ExprKind::Field {
                    value: inner,
                    index,
                } => {
                    let mut fields = vec![*index];
                    fields.extend(&key.fields);
                    let variants = key
                        .variants
                        .iter()
                        .map(|(at, ty)| (at + 1, ty))
                        .collect::<Vec<_>>();
                    key = ShapeKey::new(&fields, &variants, &mut self.flow, value.span)?;
                    base = inner;
                }
                ExprKind::Block(block) => {
                    return self.block_shape_source(base, block, &key, depth + 1);
                }
                ExprKind::Coerce { value: inner } => {
                    if let Some(record) = Self::origin_record(&base.ty)
                        && Self::origin_record(&inner.ty) != Some(record)
                        && let Type::Union(members) = &inner.ty
                    {
                        if !self.flow.spend(members.len()) {
                            return Err(Diagnostic::unsupported(
                                "proof record shape source budget exhausted",
                                value.span,
                            ));
                        }
                        let Some(member) = members
                            .iter()
                            .find(|ty| Self::origin_record(ty) == Some(record))
                        else {
                            return Ok(Snapshot::default());
                        };
                        let mut variants = vec![(0, member)];
                        variants.extend(key.variants.iter().map(|(at, ty)| (*at, ty)));
                        key = ShapeKey::new(&key.fields, &variants, &mut self.flow, value.span)?;
                        base = inner;
                        continue;
                    }

                    if inner.ty == Type::Null {
                        return Ok(Snapshot::empty());
                    }
                    if Self::origin_record(&base.ty).is_some()
                        && Self::origin_record(&base.ty) == Self::origin_record(&inner.ty)
                    {
                        base = inner;
                        continue;
                    }
                    if let Some(record) = Self::origin_record(&inner.ty) {
                        let Some(((0, selected), remaining)) = key.variants.split_first() else {
                            return Ok(Snapshot::default());
                        };
                        let Type::Union(members) = &base.ty else {
                            return Ok(Snapshot::default());
                        };
                        if !self.flow.spend(members.len()) {
                            return Err(Diagnostic::unsupported(
                                "proof record shape source budget exhausted",
                                value.span,
                            ));
                        }
                        if !members
                            .iter()
                            .any(|ty| Self::origin_record(ty) == Some(record))
                        {
                            return Ok(Snapshot::default());
                        }
                        if Self::origin_record(selected) != Some(record) {
                            return Ok(Snapshot::empty());
                        }
                        if !remaining.is_empty() {
                            let variants = remaining
                                .iter()
                                .map(|(at, ty)| (*at, ty))
                                .collect::<Vec<_>>();
                            key =
                                ShapeKey::new(&key.fields, &variants, &mut self.flow, value.span)?;
                            base = inner;
                            continue;
                        }
                        return Ok(Snapshot {
                            origins: self.record_source_origins(inner, &key.fields)?,
                            cells: self.record_source_cells(inner, &key.fields)?,
                        });
                    }
                    base = inner;
                }
                _ => return Ok(Snapshot::default()),
            }
        }
        Err(Diagnostic::unsupported(
            "proof record shape source depth exhausted",
            value.span,
        ))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod projections;

mod blocks;
