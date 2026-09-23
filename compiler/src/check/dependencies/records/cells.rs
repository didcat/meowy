use super::sources::RecordSource;
use super::{Checker, Diagnostic, Expr, Result};
use crate::check::dependencies::{Cells, references::MAX_ROOTS};

impl Checker {
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
