use super::{Checker, Diagnostic, Expr, Input, MAX_DEPTH, MAX_FIELDS, Origins, Result, Type};
use crate::check::dependencies::{Cells, references::MAX_ROOTS};

impl Checker {
    pub(super) fn call_record_view_origins(
        &mut self,
        arg: &Expr,
        path: &[usize],
        ty: &Type,
        result: &Type,
        layers: usize,
    ) -> Result<Input> {
        let cells = if path.is_empty() {
            self.reference_cell(arg)?
        } else {
            self.record_source_cells(arg, path)?
        };
        self.call_record_cell_origins(cells, arg, ty, result, layers, 0)
    }

    pub(super) fn call_record_cell_origins(
        &mut self,
        mut cells: Cells,
        arg: &Expr,
        ty: &Type,
        result: &Type,
        layers: usize,
        level: usize,
    ) -> Result<Input> {
        for _ in 0..layers {
            cells = self.expand_reference_cells(cells, arg)?;
        }
        let mut origins = Origins {
            complete: true,
            ..Origins::default()
        };
        let mut found = false;
        let mut views = vec![(ty, cells, level)];
        let mut visits = 0;
        while let Some((ty, cells, depth)) = views.pop() {
            if depth > MAX_DEPTH || !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "proof record call depth exhausted",
                    arg.span,
                ));
            }
            origins.complete &= cells.complete;
            if !crate::borrow_contract::projections(ty, result, &mut self.flow, arg.span)?
                .is_empty()
            {
                found = true;
                if !self.flow.spend(cells.places.len()) {
                    return Err(Diagnostic::unsupported(
                        "proof record call budget exhausted",
                        arg.span,
                    ));
                }
                origins
                    .roots
                    .extend(cells.places.iter().map(|(root, _)| *root));
                if origins.roots.len() > MAX_ROOTS {
                    return Err(Diagnostic::unsupported(
                        "proof reference origin capacity exhausted",
                        arg.span,
                    ));
                }
            }
            let mut pending = vec![(ty.pointee().unwrap(), Vec::new())];
            while let Some((ty, path)) = pending.pop() {
                visits += 1;
                if visits > MAX_FIELDS
                    || depth + path.len() > MAX_DEPTH
                    || !self.flow.spend(path.len() + 1)
                {
                    return Err(Diagnostic::unsupported(
                        "proof record call budget exhausted",
                        arg.span,
                    ));
                }
                if !ty.has_borrowed() {
                    continue;
                }
                if matches!(ty, Type::Union(_)) && Self::origin_record(ty).is_none() {
                    let source = match self.call_union_location_origins(
                        &cells,
                        arg,
                        ty,
                        &path,
                        result,
                        depth + 1,
                    )? {
                        Input::Unsupported => return Ok(Input::Unsupported),
                        Input::Absent => continue,
                        Input::Known(source) => source,
                    };
                    found = true;
                    origins.complete &= source.complete;
                    origins.roots.extend(source.roots);
                    if origins.roots.len() > MAX_ROOTS {
                        return Err(Diagnostic::unsupported(
                            "proof reference origin capacity exhausted",
                            arg.span,
                        ));
                    }
                    continue;
                }
                let (ty, layers) = match Self::origin_record(ty).unwrap_or(ty) {
                    Type::Record { primary, fields } if !primary.has_borrowed() => {
                        if pending.len() + views.len() + fields.len() > MAX_FIELDS {
                            return Err(Diagnostic::unsupported(
                                "proof record call capacity exhausted",
                                arg.span,
                            ));
                        }
                        for (index, field) in fields.iter().enumerate() {
                            let mut path = path.clone();
                            path.push(index);
                            pending.push((&field.ty, path));
                        }
                        continue;
                    }
                    Type::Reference(_) => {
                        let Some(view) = self.call_origin_view(ty, arg)? else {
                            return Ok(Input::Unsupported);
                        };
                        view
                    }
                    _ => return Ok(Input::Unsupported),
                };
                let nested = ty.pointee().is_some_and(Type::has_borrowed);
                if !nested
                    && crate::borrow_contract::projections(ty, result, &mut self.flow, arg.span)?
                        .is_empty()
                {
                    continue;
                }
                let mut fields = self.call_field_cells(&cells, &path, arg)?;
                if nested {
                    for _ in 0..=layers {
                        fields = self.expand_reference_cells(fields, arg)?;
                    }
                    if views.len() + pending.len() >= MAX_FIELDS {
                        return Err(Diagnostic::unsupported(
                            "proof record call capacity exhausted",
                            arg.span,
                        ));
                    }
                    views.push((ty, fields, depth + path.len()));
                    continue;
                }
                found = true;
                let source = self.call_stored_origins(fields, arg, layers)?;
                origins.complete &= source.complete;
                origins.roots.extend(source.roots);
                if origins.roots.len() > MAX_ROOTS {
                    return Err(Diagnostic::unsupported(
                        "proof reference origin capacity exhausted",
                        arg.span,
                    ));
                }
            }
        }
        Ok(if found {
            Input::Known(origins)
        } else {
            Input::Absent
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod carriers;

#[cfg(test)]
mod chains;

#[cfg(test)]
mod nested;

#[cfg(test)]
mod nullable;
