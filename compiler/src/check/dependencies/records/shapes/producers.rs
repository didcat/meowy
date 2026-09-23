use super::{Checker, Diagnostic, Expr, MAX_DEPTH, Result, ShapeKey, Snapshot, Type};
use crate::hir::ExprKind;

impl Snapshot {
    pub(super) fn empty() -> Self {
        let mut value = Self::default();
        value.origins.complete = true;
        value.cells.complete = true;
        value
    }
}

impl Checker {
    pub(super) fn record_shape_source(&mut self, value: &Expr, key: &ShapeKey) -> Result<Snapshot> {
        let mut base = value;
        for _ in 0..MAX_DEPTH {
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
                        .and_then(|shapes| shapes.get(key));
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
                ExprKind::Coerce { value: inner } => {
                    if inner.ty == Type::Null {
                        return Ok(Snapshot::empty());
                    }
                    if let Some(record) = Self::origin_record(&inner.ty) {
                        let [(0, selected)] = key.variants.as_slice() else {
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
