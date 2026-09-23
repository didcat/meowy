use super::{Cells, Checker, Diagnostic, Expr, MAX_DEPTH, MAX_FIELDS, MAX_ROOTS, Result, Type};
use crate::borrow_value::Projection;

impl Checker {
    pub(super) fn returned_record_cells(
        &mut self,
        locations: Cells,
        ty: &Type,
        expr: &Expr,
        result_depth: usize,
    ) -> Result<Option<Cells>> {
        let mut cells = Cells {
            complete: true,
            ..Cells::default()
        };
        let mut views = vec![(ty, locations, 0)];
        let mut visits = 0;
        while let Some((ty, locations, depth)) = views.pop() {
            if depth > MAX_DEPTH || !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "proof returned record cell depth exhausted",
                    expr.span,
                ));
            }
            cells.complete &= locations.complete;
            let paths =
                crate::borrow_contract::projections(ty, &expr.ty, &mut self.flow, expr.span)?;
            if paths.len() > MAX_FIELDS {
                return Err(Diagnostic::unsupported(
                    "proof returned record cell capacity exhausted",
                    expr.span,
                ));
            }
            for path in paths {
                let mut fields = Vec::new();
                for step in path {
                    let Projection::Field(index) = step else {
                        return Ok(None);
                    };
                    fields.push(index);
                }
                let source = self.call_field_cells(&locations, &fields, expr)?;
                self.merge_returned_cells(&mut cells, source, expr)?;
            }
            let mut pending = vec![(ty.pointee().unwrap(), Vec::new())];
            while let Some((ty, path)) = pending.pop() {
                visits += 1;
                if visits > MAX_FIELDS
                    || depth + path.len() > MAX_DEPTH
                    || !self.flow.spend(path.len() + 1)
                {
                    return Err(Diagnostic::unsupported(
                        "proof returned record cell budget exhausted",
                        expr.span,
                    ));
                }
                if !ty.has_borrowed() {
                    continue;
                }
                if let Some(Type::Record { primary, fields }) = Self::origin_record(ty) {
                    if primary.has_borrowed() {
                        return Ok(None);
                    }
                    if pending.len() + views.len() + fields.len() > MAX_FIELDS {
                        return Err(Diagnostic::unsupported(
                            "proof returned record cell capacity exhausted",
                            expr.span,
                        ));
                    }
                    for (index, field) in fields.iter().enumerate() {
                        let mut path = path.clone();
                        path.push(index);
                        pending.push((&field.ty, path));
                    }
                    continue;
                }
                let record = self
                    .call_shared_view(ty, expr)?
                    .filter(|(view, _)| view.pointee().is_some_and(Type::has_borrowed));
                if let Some((view, layers)) = record {
                    let mut source = self.call_field_cells(&locations, &path, expr)?;
                    for _ in 0..=layers {
                        source = self.expand_reference_cells(source, expr)?;
                    }
                    if views.len() + pending.len() >= MAX_FIELDS {
                        return Err(Diagnostic::unsupported(
                            "proof returned record cell capacity exhausted",
                            expr.span,
                        ));
                    }
                    views.push((view, source, depth + path.len()));
                    continue;
                }
                let Some(field_depth) = self.shared_cell_depth(ty, expr)? else {
                    return Ok(None);
                };
                if field_depth < result_depth {
                    continue;
                }
                let layers = field_depth - result_depth;
                let mut ty = ty;
                for _ in 0..layers {
                    ty = ty.pointee().unwrap();
                }
                if !crate::borrow_contract::returns::candidate(&expr.ty, ty) {
                    continue;
                }
                let mut source = self.call_field_cells(&locations, &path, expr)?;
                for _ in 0..=layers {
                    source = self.expand_reference_cells(source, expr)?;
                }
                self.merge_returned_cells(&mut cells, source, expr)?;
            }
        }
        Ok(Some(cells))
    }

    pub(super) fn merge_returned_cells(
        &mut self,
        cells: &mut Cells,
        source: Cells,
        expr: &Expr,
    ) -> Result<()> {
        let work = source
            .places
            .iter()
            .map(|(_, path)| path.len() + 1)
            .sum::<usize>()
            + 1;
        if !self.flow.spend(work) {
            return Err(Diagnostic::unsupported(
                "proof returned record cell budget exhausted",
                expr.span,
            ));
        }
        cells.complete &= source.complete;
        cells.places.extend(source.places);
        if cells.places.len() > MAX_ROOTS {
            return Err(Diagnostic::unsupported(
                "proof returned record cell capacity exhausted",
                expr.span,
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod chains;

#[cfg(test)]
mod nested;
