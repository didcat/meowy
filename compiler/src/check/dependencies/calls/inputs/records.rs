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
        let mut cells = if path.is_empty() {
            self.reference_cell(arg)?
        } else {
            self.record_source_cells(arg, path)?
        };
        for _ in 0..layers {
            cells = self.expand_reference_cells(cells, arg)?;
        }
        let mut origins = Origins {
            complete: cells.complete,
            ..Origins::default()
        };
        let mut found =
            !crate::borrow_contract::projections(ty, result, &mut self.flow, arg.span)?.is_empty();
        if found {
            if !self.flow.spend(cells.places.len()) {
                return Err(Diagnostic::unsupported(
                    "proof record call budget exhausted",
                    arg.span,
                ));
            }
            origins
                .roots
                .extend(cells.places.iter().map(|(root, _)| *root));
        }
        let mut pending = vec![(ty.pointee().unwrap(), Vec::new())];
        let mut visits = 0;
        while let Some((ty, path)) = pending.pop() {
            visits += 1;
            if visits > MAX_FIELDS || path.len() > MAX_DEPTH || !self.flow.spend(path.len() + 1) {
                return Err(Diagnostic::unsupported(
                    "proof record call budget exhausted",
                    arg.span,
                ));
            }
            if !ty.has_borrowed() {
                continue;
            }
            let (ty, layers) = match Self::origin_record(ty).unwrap_or(ty) {
                Type::Record { primary, fields } if !primary.has_borrowed() => {
                    if pending.len() + fields.len() > MAX_FIELDS {
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
                    let Some(view) = self.call_shared_view(ty, arg)? else {
                        return Ok(Input::Unsupported);
                    };
                    if view.0.pointee().is_some_and(Type::has_borrowed) {
                        return Ok(Input::Unsupported);
                    }
                    view
                }
                _ => return Ok(Input::Unsupported),
            };
            if crate::borrow_contract::projections(ty, result, &mut self.flow, arg.span)?.is_empty()
            {
                continue;
            }
            found = true;
            for (root, base) in &cells.places {
                if base.len() + path.len() > MAX_DEPTH
                    || !self.flow.spend(base.len() + path.len() + 1)
                {
                    return Err(Diagnostic::unsupported(
                        "proof record call path budget exhausted",
                        arg.span,
                    ));
                }
                let mut field = base.clone();
                field.extend(&path);
                let source = self.call_stored_origins(
                    Cells {
                        places: std::collections::BTreeSet::from([(*root, field)]),
                        complete: true,
                    },
                    arg,
                    layers,
                )?;
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
