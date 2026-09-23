use super::{Checker, Diagnostic, Expr, Result};
use crate::check::dependencies::{Cells, records::MAX_DEPTH};

impl Checker {
    pub(super) fn call_field_cells(
        &mut self,
        cells: &Cells,
        path: &[usize],
        expr: &Expr,
    ) -> Result<Cells> {
        let mut fields = Cells {
            complete: cells.complete,
            ..Cells::default()
        };
        for (root, base) in &cells.places {
            if base.len() + path.len() > MAX_DEPTH || !self.flow.spend(base.len() + path.len() + 1)
            {
                return Err(Diagnostic::unsupported(
                    "proof record call path budget exhausted",
                    expr.span,
                ));
            }
            let mut field = base.clone();
            field.extend(path);
            fields.places.insert((*root, field));
        }
        Ok(fields)
    }
}
