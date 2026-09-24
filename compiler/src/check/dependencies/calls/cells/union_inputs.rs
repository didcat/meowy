use super::{Cells, Checker, Expr, Result, Type};

impl Checker {
    pub(super) fn unmatched_union_layers(
        &mut self,
        input: &Type,
        result: &Type,
        depth: usize,
        expr: &Expr,
    ) -> Result<Option<usize>> {
        let Some(layers) = self.shared_cell_depth(input, expr)? else {
            return Ok(None);
        };
        Ok(Self::unmatched_union_terminal(input, layers, result, depth).then_some(layers - 1))
    }

    pub(super) fn union_input_cells(
        &mut self,
        input: &Expr,
        result: &Type,
        depth: usize,
        layers: usize,
    ) -> Result<Option<Cells>> {
        let mut locations = self.reference_cell_at(input, depth + 1)?;
        let mut ty = &input.ty;
        for _ in 0..layers {
            locations = self.expand_reference_cells(locations, input)?;
            ty = ty.pointee().unwrap();
        }
        let Some(mut cells) =
            self.hidden_union_cells(&locations, &[], ty.pointee().unwrap(), result, input)?
        else {
            return Ok(None);
        };
        cells.complete &= locations.complete;
        Ok(Some(cells))
    }
}

#[cfg(test)]
mod tests;
