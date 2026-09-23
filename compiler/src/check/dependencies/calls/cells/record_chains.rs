use super::{Cells, Checker, Expr, Result, Type};

impl Checker {
    pub(super) fn record_chain_cells(
        &mut self,
        mut locations: Cells,
        mut ty: &Type,
        layers: usize,
        expr: &Expr,
    ) -> Result<(Cells, Cells)> {
        let mut matched = Cells {
            complete: true,
            ..Cells::default()
        };
        for layer in 0..=layers {
            self.origin_visit(expr)?;
            if crate::borrow_contract::returns::candidate(&expr.ty, ty) {
                self.merge_returned_cells(&mut matched, locations.clone(), expr)?;
            }
            if layer < layers {
                locations = self.expand_reference_cells(locations, expr)?;
                ty = ty.pointee().unwrap();
            }
        }
        Ok((matched, locations))
    }
}

#[cfg(test)]
mod tests;
