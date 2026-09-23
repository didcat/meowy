use super::{Cells, Checker, Diagnostic, Expr, MAX_DEPTH, MAX_FIELDS, Result, Type};

impl Checker {
    pub(super) fn call_record_locations(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        depth: usize,
    ) -> Result<Cells> {
        if !self.flow.spend(args.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof returned record location budget exhausted",
                expr.span,
            ));
        }
        let mut cells = Cells::default();
        let mut found = false;
        let mut pending = args
            .iter()
            .map(|arg| (arg, &arg.ty, Vec::new()))
            .collect::<Vec<_>>();
        let mut visits = 0;
        while let Some((arg, ty, path)) = pending.pop() {
            visits += 1;
            if visits > MAX_FIELDS || path.len() > MAX_DEPTH || !self.flow.spend(path.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "proof returned record input budget exhausted",
                    expr.span,
                ));
            }
            if !ty.has_borrowed() {
                continue;
            }
            if let Some(Type::Record { primary, fields }) = Self::origin_record(ty) {
                if primary.has_borrowed() {
                    return Ok(Cells::default());
                }
                if pending.len() + fields.len() > MAX_FIELDS {
                    return Err(Diagnostic::unsupported(
                        "proof returned record input capacity exhausted",
                        expr.span,
                    ));
                }
                for (index, field) in fields.iter().enumerate() {
                    let mut path = path.clone();
                    path.push(index);
                    pending.push((arg, &field.ty, path));
                }
                continue;
            }
            let Some((view, layers)) = self.call_shared_view(ty, expr)? else {
                return Ok(Cells::default());
            };
            let exact = crate::borrow_contract::returns::candidate(&expr.ty, view);
            if !exact && !view.pointee().is_some_and(Type::has_borrowed) {
                continue;
            }
            let mut source = if path.is_empty() {
                self.reference_cell_at(arg, depth + 1)?
            } else {
                self.record_source_cells_at(arg, &path, depth + 1)?
            };
            for _ in 0..layers {
                source = self.expand_reference_cells(source, expr)?;
            }
            if !exact {
                let Some(projected) =
                    self.returned_record_cells(source, view, expr, None, &expr.ty)?
                else {
                    return Ok(Cells::default());
                };
                source = projected;
            }
            if !found {
                cells.complete = true;
                found = true;
            }
            self.merge_returned_cells(&mut cells, source, expr)?;
        }
        Ok(cells)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod records;

#[cfg(test)]
mod chains;

#[cfg(test)]
mod projections;

#[cfg(test)]
mod nullable;
