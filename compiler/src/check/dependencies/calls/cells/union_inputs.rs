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
        ty: &Type,
        path: &[usize],
        result: &Type,
        depth: usize,
        layers: usize,
    ) -> Result<Option<Cells>> {
        let locations = if path.is_empty() {
            self.reference_cell_at(input, depth + 1)?
        } else {
            self.record_source_cells_at(input, path, depth + 1)?
        };
        self.union_location_cells(locations, ty, result, layers, input)
    }

    pub(super) fn union_location_cells(
        &mut self,
        locations: Cells,
        ty: &Type,
        result: &Type,
        layers: usize,
        input: &Expr,
    ) -> Result<Option<Cells>> {
        self.union_location_cells_at(locations, ty, result, layers, input, 0)
    }

    pub(super) fn union_location_cells_at(
        &mut self,
        mut locations: Cells,
        mut ty: &Type,
        result: &Type,
        layers: usize,
        input: &Expr,
        level: usize,
    ) -> Result<Option<Cells>> {
        for _ in 0..layers {
            locations = self.expand_reference_cells(locations, input)?;
            ty = ty.pointee().unwrap();
        }
        let Some(mut cells) = self.hidden_union_cells_at(
            &locations,
            &[],
            ty.pointee().unwrap(),
            result,
            input,
            level,
        )?
        else {
            return Ok(None);
        };
        cells.complete &= locations.complete;
        Ok(Some(cells))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod stored;

#[cfg(test)]
mod borrowed;
