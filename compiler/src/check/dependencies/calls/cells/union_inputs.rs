use super::{Cells, Checker, Expr, Result, Type};

impl Checker {
    pub(super) fn unmatched_union_view(input: &Type, result: &Type, depth: usize) -> bool {
        matches!(input, Type::Reference(target) if matches!(target.as_ref(), Type::Union(_)))
            && Self::unmatched_union_terminal(input, 1, result, depth)
    }

    pub(super) fn union_input_cells(
        &mut self,
        input: &Expr,
        result: &Type,
        depth: usize,
    ) -> Result<Option<Cells>> {
        let locations = self.reference_cell_at(input, depth + 1)?;
        let Some(mut cells) =
            self.hidden_union_cells(&locations, &[], input.ty.pointee().unwrap(), result, input)?
        else {
            return Ok(None);
        };
        cells.complete &= locations.complete;
        Ok(Some(cells))
    }
}

#[cfg(test)]
mod tests;
