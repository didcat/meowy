use super::sources::RecordSource;
use super::{Checker, Diagnostic, Expr, Result};
use crate::check::dependencies::{Cells, references::MAX_ROOTS};

impl Checker {
    pub(crate) fn merge_record_cells(
        &mut self,
        id: usize,
        path: &[usize],
        mut cells: Cells,
        merge: bool,
        span: crate::ast::Span,
    ) -> Result<Cells> {
        let prior = merge
            .then(|| {
                self.record_cells
                    .get(&id)
                    .and_then(|fields| fields.get(path))
            })
            .flatten();
        let work = cells
            .places
            .iter()
            .chain(prior.into_iter().flat_map(|cells| &cells.places))
            .map(|(_, path)| path.len() + 1)
            .sum::<usize>()
            + 1;
        if path.len() > super::MAX_DEPTH
            || path.iter().any(|index| *index >= super::MAX_FIELDS)
            || !self.flow.spend(work)
        {
            return Err(Diagnostic::unsupported(
                "proof record cell budget exhausted",
                span,
            ));
        }
        if let Some(prior) = prior {
            cells.complete &= prior.complete;
            cells.places.extend(prior.places.iter().cloned());
        } else if merge {
            cells.complete = false;
        }
        if cells.places.len() > MAX_ROOTS {
            return Err(Diagnostic::unsupported(
                "proof record cell capacity exhausted",
                span,
            ));
        }
        Ok(cells)
    }

    pub(crate) fn write_carrier_field(
        &mut self,
        id: usize,
        path: &[usize],
        value: &Expr,
    ) -> Result<()> {
        if !self.origin_carrier(&value.ty, value.span)? {
            return Ok(());
        }
        let cells = self.reference_cell(value)?;
        let cells = self.merge_record_cells(id, path, cells, true, value.span)?;
        let fields = self.record_cells.entry(id).or_default();
        if fields.len() == super::MAX_FIELDS && !fields.contains_key(path) {
            return Err(Diagnostic::unsupported(
                "proof record cell capacity exhausted",
                value.span,
            ));
        }
        fields.insert(path.to_vec(), cells);
        Ok(())
    }

    pub(crate) fn track_record_cells(
        &mut self,
        id: usize,
        prefix: &[usize],
        value: &Expr,
        merge: bool,
    ) -> Result<()> {
        let paths = self.record_paths_for(value, true)?;
        let mut fields = std::collections::BTreeMap::new();
        if !prefix.is_empty()
            && let Some(prior) = self.record_cells.get(&id)
        {
            let work = prior
                .iter()
                .map(|(path, cells)| {
                    path.len()
                        + cells
                            .places
                            .iter()
                            .map(|(_, path)| path.len() + 1)
                            .sum::<usize>()
                        + 1
                })
                .sum();
            if !self.flow.spend(work) {
                return Err(Diagnostic::unsupported(
                    "proof record cell budget exhausted",
                    value.span,
                ));
            }
            fields = prior.clone();
        }
        for path in paths {
            let mut key = prefix.to_vec();
            key.extend(&path);
            let cells = self.record_source_cells(value, &path)?;
            let cells = self.merge_record_cells(id, &key, cells, merge, value.span)?;
            fields.insert(key, cells);
        }
        if fields.len() > super::MAX_FIELDS {
            return Err(Diagnostic::unsupported(
                "proof record cell capacity exhausted",
                value.span,
            ));
        }
        if !fields.is_empty() {
            self.record_cells.insert(id, fields);
        }
        Ok(())
    }

    pub(crate) fn record_source_cells(&mut self, value: &Expr, path: &[usize]) -> Result<Cells> {
        let sources = match self.record_source_locations(value, path)? {
            RecordSource::Unknown => return Ok(Cells::default()),
            RecordSource::Empty => {
                return Ok(Cells {
                    complete: true,
                    ..Cells::default()
                });
            }
            RecordSource::Direct(place) => {
                return Ok(self
                    .record_cells
                    .get(&place.root)
                    .and_then(|fields| fields.get(&place.fields))
                    .cloned()
                    .unwrap_or_default());
            }
            RecordSource::Alternatives(sources) => sources,
        };
        let mut cells = Cells {
            complete: true,
            ..Cells::default()
        };
        for place in sources {
            let work = self
                .stored_cells(place.root, &place.fields)
                .map_or(0, |source| {
                    source.places.iter().map(|(_, path)| path.len() + 1).sum()
                })
                + 1;
            if !self.flow.spend(work) {
                return Err(Diagnostic::unsupported(
                    "proof record cell budget exhausted",
                    value.span,
                ));
            }
            let source = self.stored_cells(place.root, &place.fields);
            cells.complete &= source.is_some_and(|source| source.complete);
            if let Some(source) = source {
                cells.places.extend(source.places.iter().cloned());
            }
            if cells.places.len() > MAX_ROOTS {
                return Err(Diagnostic::unsupported(
                    "proof record cell capacity exhausted",
                    value.span,
                ));
            }
        }
        Ok(cells)
    }
}

#[cfg(test)]
mod tests;
