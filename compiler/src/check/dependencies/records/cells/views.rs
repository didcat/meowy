use super::{Cells, Checker, Diagnostic, Expr, MAX_ROOTS, Result};
use crate::hir::Type;

impl Checker {
    pub(super) fn record_view_field_cells(
        &mut self,
        view: &Expr,
        path: &[usize],
        depth: usize,
    ) -> Result<Cells> {
        let Type::Reference(target) = &view.ty else {
            return Ok(Cells::default());
        };
        if Self::origin_record(target).is_none() || path.is_empty() {
            return Ok(Cells::default());
        }
        let locations = self.reference_cell_at(view, depth + 1)?;
        if locations.places.len() > MAX_ROOTS {
            return Err(Diagnostic::unsupported(
                "proof record view location capacity exhausted",
                view.span,
            ));
        }
        let mut cells = Cells {
            complete: locations.complete,
            ..Cells::default()
        };
        for (root, mut prefix) in locations.places {
            let Some(ty) = self.record_view_field_type(view, root, &prefix, path)? else {
                cells.complete = false;
                continue;
            };
            if !matches!(ty, Type::Reference(_)) || !self.origin_carrier(ty, view.span)? {
                cells.complete = false;
                continue;
            }
            prefix.extend(path);
            let work = self.stored_cells(root, &prefix).map_or(0, |source| {
                source
                    .places
                    .iter()
                    .map(|(_, path)| path.len() + 1)
                    .sum::<usize>()
            }) + 1;
            if !self.flow.spend(work) {
                return Err(Diagnostic::unsupported(
                    "proof record view cell budget exhausted",
                    view.span,
                ));
            }
            let source = self.stored_cells(root, &prefix);
            cells.complete &= source.is_some_and(|source| source.complete);
            if let Some(source) = source {
                cells.places.extend(source.places.iter().cloned());
            }
            if cells.places.len() > MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof record view cell capacity exhausted",
                    view.span,
                ));
            }
        }
        Ok(cells)
    }
}

#[cfg(test)]
mod tests;
