use super::{Checker, Diagnostic, Expr, MAX_DEPTH, Result, ShapeKey, Snapshot, Type};
use crate::check::dependencies::{Cells, references::MAX_ROOTS};
use crate::hir::ExprKind;
use std::collections::BTreeSet;

impl Checker {
    pub(super) fn view_shape_source(&mut self, view: &Expr, key: &ShapeKey) -> Result<Snapshot> {
        if !matches!(view.ty, Type::Reference(_)) {
            return Ok(Snapshot::default());
        }
        let (locations, suffix) = if let Some(place) = self.shape_temporary(view)? {
            (
                Cells {
                    places: BTreeSet::from([(place.root, place.fields)]),
                    complete: true,
                },
                Vec::new(),
            )
        } else {
            let mut base = view;
            let mut suffix = Vec::new();
            let mut depth = 0;
            while let ExprKind::Reborrow {
                value: inner,
                fields,
                ..
            } = &base.kind
            {
                if depth == MAX_DEPTH
                    || suffix.len() + fields.len() > MAX_DEPTH
                    || !self.flow.spend(fields.len() + 1)
                {
                    return Err(Diagnostic::unsupported(
                        "proof union view path budget exhausted",
                        view.span,
                    ));
                }
                suffix.extend(fields.iter().rev().copied());
                base = inner;
                depth += 1;
            }
            suffix.reverse();
            (self.reference_cell(base)?, suffix)
        };
        if locations.places.len() > MAX_ROOTS {
            return Err(Diagnostic::unsupported(
                "proof union view location capacity exhausted",
                view.span,
            ));
        }
        let mut result = Snapshot::empty();
        result.origins.complete = locations.complete;
        result.cells.complete = locations.complete;
        for (root, mut prefix) in locations.places {
            prefix.extend(&suffix);
            if prefix.len() > MAX_DEPTH || !self.flow.spend(prefix.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "proof union view prefix budget exhausted",
                    view.span,
                ));
            }
            let mut ty = self.locals.get(root);
            for index in &prefix {
                ty = ty.and_then(Self::origin_record).and_then(|ty| {
                    let Type::Record { fields, .. } = ty else {
                        return None;
                    };
                    fields.get(*index).map(|field| &field.ty)
                });
            }
            if ty.is_none() {
                result.origins.complete = false;
                result.cells.complete = false;
                continue;
            }
            let mut fields = prefix.clone();
            fields.extend(&key.fields);
            let variants = key
                .variants
                .iter()
                .map(|(at, ty)| (at + prefix.len(), ty))
                .collect::<Vec<_>>();
            let key = ShapeKey::new(&fields, &variants, &mut self.flow, view.span)?;
            let next = self.stored_shape_source(root, &key, view.span)?;
            result.merge(next, &mut self.flow, view.span)?;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
